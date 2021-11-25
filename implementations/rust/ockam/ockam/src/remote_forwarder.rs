#![deny(missing_docs)]

use crate::{route, Context, Message, OckamError};
use ockam_core::compat::rand::random;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Address, Any, LocalMessage, Result, Route, Routed, TransportMessage, Worker};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Information about a remotely forwarded worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Message)]
pub struct RemoteForwarderInfo {
    forwarding_route: Route,
    remote_address: String,
    worker_address: Address,
}

impl RemoteForwarderInfo {
    /// Returns the forwarding route.
    pub fn forwarding_route(&self) -> &Route {
        &self.forwarding_route
    }
    /// Returns the remote address.
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }
    /// Returns the worker address.
    pub fn worker_address(&self) -> &Address {
        &self.worker_address
    }
}

/// This Worker is responsible for registering on Ockam Hub and forwarding messages to local Worker
pub struct RemoteForwarder {
    route: Route,
    destination: Route,
    callback_address: Address,
    register_payload: String,
}

impl RemoteForwarder {
    fn new(
        hub_addr: impl Into<Address>,
        service_address: impl Into<Address>,
        destination: impl Into<Address>,
        callback_address: Address,
        register_payload: String,
    ) -> Self {
        Self {
            route: route![hub_addr, service_address],
            destination: route![destination],
            callback_address,
            register_payload,
        }
    }

    /// Create and start new RemoteForwarder with given Ockam Hub address
    /// and Address of destination Worker that should receive forwarded messages
    pub async fn create(
        ctx: &Context,
        hub_addr: impl Into<Address>,
        destination: impl Into<Address>,
    ) -> Result<RemoteForwarderInfo> {
        let address: Address = random();
        let forwarder = Self::new(
            hub_addr,
            "forwarding_service",
            destination,
            address,
            "register".to_string(),
        );
        Self::start_worker(ctx, forwarder).await
    }

    /// Create and start new pub_sub RemoteForwarder
    /// hub_addr - address to a transport or connection to hub
    /// destination - address to a worker that should receive forwarded messages
    /// name - pub_sub subscription name
    /// topic - pub_sub topic to subscribe to
    pub async fn create_pub_sub(
        ctx: &Context,
        hub_addr: impl Into<Address>,
        destination: impl Into<Address>,
        name: impl Into<String>,
        topic: impl Into<String>,
    ) -> Result<RemoteForwarderInfo> {
        let address: Address = random();
        // TODO: there should be a better way to concat strings than this
        let mut register_payload = String::new();
        register_payload += &name.into();
        register_payload += ":";
        register_payload += &topic.into();
        let forwarder = Self::new(
            hub_addr,
            "pub_sub_service",
            destination,
            address,
            register_payload,
        );
        Self::start_worker(ctx, forwarder).await
    }

    async fn start_worker(
        ctx: &Context,
        forwarder: RemoteForwarder,
    ) -> Result<RemoteForwarderInfo> {
        let address = forwarder.callback_address.clone();
        let mut child_ctx = ctx.new_context(address).await?;

        let worker_address: Address = random();
        debug!("Starting RemoteForwarder at {}", &worker_address);
        ctx.start_worker(worker_address, forwarder).await?;

        let resp = child_ctx
            .receive::<RemoteForwarderInfo>()
            .await?
            .take()
            .body();

        Ok(resp)
    }
}

#[crate::worker]
impl Worker for RemoteForwarder {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("RemoteForwarder registering...");
        let register_payload = &self.register_payload;
        ctx.send(self.route.clone(), register_payload.clone())
            .await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.return_route();
        let resp = resp.body();
        // TODO: we might want to support a different format for response
        // other than the same payload as request
        if resp.as_str() == register_payload {
            self.route = route.clone();
        } else {
            return Err(OckamError::InvalidHubResponse.into());
        }
        info!("RemoteForwarder registered with route: {}", route);
        let address;
        if let Some(a) = route.clone().recipient().to_string().strip_prefix("0#") {
            address = a.to_string();
        } else {
            return Err(OckamError::InvalidHubResponse.into());
        }

        ctx.send(
            self.callback_address.clone(),
            RemoteForwarderInfo {
                forwarding_route: route,
                remote_address: address,
                worker_address: ctx.address(),
            },
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let payload = msg.into_transport_message().payload;
        debug!("RemoteForwarder received message");

        let msg = TransportMessage::v1(self.destination.clone(), return_route, payload);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(())
    }
}
