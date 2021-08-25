#![deny(missing_docs)]

use crate::{route, Context, OckamError};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Address, Any, LocalMessage, Result, Route, Routed, TransportMessage, Worker};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[cfg(not(feature = "std"))]
use ockam_core::compat::rand::random;
#[cfg(feature = "std")]
use rand::random;

/// Information about a remotely forwarded worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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
}

impl RemoteForwarder {
    fn new(
        hub_addr: impl Into<Address>,
        destination: impl Into<Address>,
        callback_address: Address,
    ) -> Self {
        Self {
            route: route![hub_addr, "forwarding_service"],
            destination: route![destination],
            callback_address,
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
        let mut child_ctx = ctx.new_context(address).await?;
        let forwarder = Self::new(hub_addr, destination, child_ctx.address());

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
        ctx.send(self.route.clone(), "register".to_string()).await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.return_route();
        let resp = resp.body();
        match resp.as_str() {
            "register" => self.route = route.clone(),
            _ => return Err(OckamError::InvalidHubResponse.into()),
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
