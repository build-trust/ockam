use minicbor::{Decode, Encode};

use ockam::remote::RemoteForwarderInfo;
use ockam_core::Route;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

use crate::CowStr;

/// Request body when instructing a node to create a forwarder
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateForwarder<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3386455>,
    /// Ockam's cloud forwarder node address
    #[b(1)] pub(crate) address: CowStr<'a>,
    /// Forwarder alias
    #[n(2)] pub(crate) alias: Option<CowStr<'a>>,
}

impl<'a> CreateForwarder<'a> {
    pub fn new(address: Route, alias: Option<&'a str>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            address: address.to_string().into(),
            alias: alias.map(|s| s.into()),
        }
    }
}

/// Response body when creating a forwarder
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ForwarderInfo<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2757430>,
    #[b(1)] forwarding_route: CowStr<'a>,
    #[b(2)] remote_address: CowStr<'a>,
    #[b(3)] worker_address: CowStr<'a>,
}

impl<'a> ForwarderInfo<'a> {
    pub fn forwarding_route(&'a self) -> &'a str {
        &self.forwarding_route
    }

    pub fn remote_address(&'a self) -> &'a str {
        &self.remote_address
    }
}

impl<'a> From<RemoteForwarderInfo> for ForwarderInfo<'a> {
    fn from(inner: RemoteForwarderInfo) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            forwarding_route: inner.forwarding_route().to_string().into(),
            remote_address: inner.remote_address().to_string().into(),
            worker_address: inner.worker_address().to_string().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use minicbor::Decoder;

    use ockam::{Context, TcpTransport, TCP};
    use ockam_core::{compat::rand::Rng, route, Address, Result, Routed, Worker};

    use crate::cloud::enroll::tests::auth0::MockAuth0Service;
    use crate::nodes::{types::*, NodeMan};
    use crate::*;

    use super::*;

    #[ockam_macros::test]
    async fn create_forwarder(ctx: &mut Context) -> Result<()> {
        let cloud_address = match std::env::var("CLOUD_ADDRESS") {
            Ok(addr) if !addr.is_empty() => addr,
            _ => {
                ctx.stop().await?;
                return Ok(());
            }
        };

        let node_dir = tempfile::tempdir().unwrap();

        // Create node manager to handle requests
        let node_manager = "manager";
        let transport = TcpTransport::create(ctx).await?;
        let node_address = transport.listen("127.0.0.1:0").await?;
        let node_man = NodeMan::create(
            ctx,
            "node".to_string(),
            node_dir.into_path(),
            (
                TransportType::Tcp,
                TransportMode::Listen,
                node_address.to_string(),
            ),
            transport,
            MockAuth0Service,
        )
        .await?;
        ctx.start_worker(node_manager, node_man).await?;

        // Start Echoer worker
        ctx.start_worker("echoer", Echoer).await?;

        // Create CreateForwarder request
        let request = {
            let route = route![(TCP, &cloud_address)];
            let mut buf = vec![];
            Request::builder(Method::Post, "/node/forwarder")
                .body(CreateForwarder::new(route, None))
                .encode(&mut buf)?;
            buf
        };

        // Send CreateForwarder request
        let forwarding_address = {
            let response: Vec<u8> = ctx.send_and_receive(node_manager, request).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let body = dec.decode::<ForwarderInfo>()?;
            body.remote_address.to_string()
        };

        // Send message to forwarder
        {
            let msg: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(256)
                .map(char::from)
                .collect();
            let mut ctx = ctx.new_detached(Address::random_local()).await?;
            let route = route![(TCP, &cloud_address), forwarding_address, "echoer"];
            ctx.send(route, msg.clone()).await?;
            let reply = ctx.receive::<String>().await?;
            assert_eq!(msg, reply.take().body());
        };

        ctx.stop().await?;
        Ok(())
    }

    struct Echoer;

    #[ockam::worker]
    impl Worker for Echoer {
        type Message = String;
        type Context = Context;

        async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
            ctx.send(msg.return_route(), msg.body()).await
        }
    }
}
