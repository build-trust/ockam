//! Node Manager (Node Man, the superhero that we deserve)

use core::convert::Infallible;

use minicbor::{encode::Write, Decoder};

use ockam::remote::RemoteForwarder;
use ockam::{Address, Context, Result, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::{Identity, TrustEveryonePolicy};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use crate::error::ApiError;
use crate::{nodes::types::*, Method, Request, Response, ResponseBuilder, Status};

use super::types::{CreateTransport, DeleteTransport};

// TODO: Move to multiaddr implementation
fn invalid_multiaddr_error() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Core, Kind::Invalid, "Invalid multiaddr")
}

// TODO: Move to multiaddr implementation
fn map_multiaddr_err(_err: ockam_multiaddr::Error) -> ockam_core::Error {
    invalid_multiaddr_error()
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan {
    node_name: String,
    api_transport_id: Address,
    transports: BTreeMap<Address, (TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,
}

impl NodeMan {
    /// Create a new NodeMan with the node name from the ockam CLI
    pub fn new(
        node_name: String,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Self {
        let api_transport_id = Address::random_local();

        let mut transports = BTreeMap::new();
        transports.insert(api_transport_id.clone(), api_transport);

        Self {
            node_name,
            api_transport_id,
            transports,
            tcp_transport,
        }
    }
}

impl NodeMan {
    fn get_transports(&self) -> Vec<TransportStatus<'_>> {
        self.transports
            .iter()
            .map(|(tid, (tt, tm, addr))| {
                TransportStatus::new(*tt, *tm, addr.clone(), tid.without_type().to_string())
            })
            .collect()
    }

    async fn add_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'_>>> {
        let CreateTransport { tt, tm, addr, .. } = dec.decode()?;

        use {TransportMode::*, TransportType::*};

        info!(
            "Handling request to create a new transport: {}, {}, {}",
            tt, tm, addr
        );
        let addr = addr.to_string();

        let res = match (tt, tm) {
            (Tcp, Listen) => self
                .tcp_transport
                .listen(&addr)
                .await
                .map(|socket| socket.to_string()),
            (Tcp, Connect) => self
                .tcp_transport
                .connect(&addr)
                .await
                .map(|ockam_addr| ockam_addr.to_string()),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok(_) => {
                let tid = Address::random_local();
                self.transports.insert(tid.clone(), (tt, tm, addr.clone()));
                Response::ok(req.id()).body(TransportStatus::new(
                    tt,
                    tm,
                    addr,
                    tid.without_type().to_string(),
                ))
            }
            Err(msg) => Response::bad_request(req.id()).body(TransportStatus::new(
                tt,
                tm,
                msg.to_string(),
                "<none>".to_string(),
            )),
        };

        Ok(response)
    }

    async fn delete_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to delete transport: {}", body.tid);

        let tid: Address = format!("0#{}", body.tid).into();

        if self.api_transport_id == tid && !body.force {
            warn!("User requested to delete the API transport without providing force OP flag...");
            return Ok(Response::bad_request(req.id()));
        }

        match self.transports.get(&tid) {
            Some(t) if t.1 == TransportMode::Listen => {
                warn!("It is not currently supported to destroy LISTEN transports");
                Ok(Response::bad_request(req.id()))
            }
            Some(t) => {
                self.tcp_transport.disconnect(&t.2).await?;
                self.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }

    async fn create_forwarder<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
        let CreateForwarder { address, alias, .. } = dec.decode()?;
        let address = Address::from_string(address.to_string());
        debug!(%address, ?alias, "Handling CreateForwarder request");
        let forwarder = match alias {
            Some(alias) => RemoteForwarder::create_static(ctx, address, alias.to_string()).await,
            None => RemoteForwarder::create(ctx, address).await,
        };
        match forwarder {
            Ok(info) => {
                let b = ForwarderInfo::from(info);
                debug!(
                    forwarding_route = %b.forwarding_route(),
                    remote_address = %b.remote_address(),
                    "CreateForwarder request processed, sending back response"
                );
                Response::ok(req.id()).body(b).encode(enc)?;
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .encode(enc)?;
            }
        };
        Ok(())
    }

    async fn create_secure_channel(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'_>>> {
        let CreateSecureChannelRequest { addr, .. } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        // TODO: Improve error handling
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        // TODO: Load Vault and Identity from the Storage. Possibly move this part from ockam_command
        let identity = Identity::create(ctx, &Vault::create()).await?;

        let channel = identity
            .create_secure_channel(route, TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;

        // TODO: Create Secure Channels Registry

        let response =
            Response::ok(req.id()).body(CreateSecureChannelResponse::new(channel.to_string()));

        Ok(response)
    }

    async fn create_secure_channel_listener(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let CreateSecureChannelListenerRequest { addr, .. } = dec.decode()?;

        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        // TODO: Should we check if Address is LOCAL?
        let addr = Address::from(addr.as_ref());

        // TODO: Load Vault and Identity from the Storage. Possibly move this part from ockam_command
        let identity = Identity::create(ctx, &Vault::create()).await?;

        identity
            .create_secure_channel_listener(addr, TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;

        // TODO: Create Secure Channel Listeners Registry

        let response = Response::ok(req.id());

        Ok(response)
    }

    async fn handle_request<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
        trace! {
            target: "ockam::nodeman::service",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use Method::*;
        let path = req.path();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        match (method, path) {
            // == Get information about this node
            (Get, "/node") => Response::ok(req.id())
                .body(NodeStatus::new(
                    self.node_name.as_str(),
                    "[✓]",
                    ctx.list_workers().await?.len() as u32,
                    std::process::id() as i32,
                    self.transports.len() as u32,
                ))
                .encode(enc)?,
            // == Get all transports
            (Get, "/node/transport") => Response::ok(req.id())
                .body(TransportList::new(self.get_transports()))
                .encode(enc)?,
            // TODO: Get all transports
            // == Create a new transport
            (Post, "/node/transport") => self.add_transport(req, dec).await?.encode(enc)?,
            (Delete, "/node/transport") => self.delete_transport(req, dec).await?.encode(enc)?,
            // == Secure channels
            (Post, "/node/secure_channel") => self
                .create_secure_channel(ctx, req, dec)
                .await?
                .encode(enc)?,
            (Post, "/node/secure_channel_listener") => self
                .create_secure_channel_listener(ctx, req, dec)
                .await?
                .encode(enc)?,
            // == Create a new forwarder
            (Post, "/node/forwarder") => self.create_forwarder(ctx, req, dec, enc).await?,
            (method, path) => {
                warn!("Called invalid endpoint: {} {}", method, path);
                todo!()
            }
        }

        Ok(())
    }
}

#[ockam::worker]
impl Worker for NodeMan {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let mut buf = vec![];
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        self.handle_request(ctx, &req, &mut dec, &mut buf).await?;
        ctx.send(msg.return_route(), buf).await
    }
}
