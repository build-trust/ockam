use crate::{Context, OckamError};
use ockam_core::lib::net::SocketAddr;
use ockam_core::{Address, Any, Result, Route, Routed, TransportMessage, Worker};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteForwarderInfo {
    forwarding_route: Route,
    remote_address: String,
    worker_address: Address,
}

impl RemoteForwarderInfo {
    pub fn forwarding_route(&self) -> &Route {
        &self.forwarding_route
    }
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }
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
    fn new(hub_addr: SocketAddr, destination: Address, callback_address: Address) -> Self {
        let route = Route::new()
            .append(format!("1#{}", hub_addr))
            .append("forwarding_service")
            .into();
        let destination = Route::new().append(destination).into();
        Self {
            route,
            destination,
            callback_address,
        }
    }

    /// Create and start new RemoteForwarder with given Ockam Hub address
    /// and Address of destination Worker that should receive forwarded messages
    pub async fn create<A: Into<Address>, S: Into<String>>(
        ctx: &Context,
        hub_addr: S,
        destination: A,
    ) -> Result<RemoteForwarderInfo> {
        if let Ok(hub_addr) = hub_addr.into().parse::<SocketAddr>() {
            let address: Address = random();
            let mut child_ctx = ctx.new_context(address).await?;
            let forwarder = Self::new(hub_addr, destination.into(), child_ctx.address());

            let worker_address: Address = random();
            debug!("Starting RemoteForwarder at {}", &worker_address);
            ctx.start_worker(worker_address, forwarder).await?;

            let resp = child_ctx
                .receive::<RemoteForwarderInfo>()
                .await?
                .take()
                .body();

            Ok(resp)
        } else {
            Err(OckamError::InvalidParameter.into())
        }
    }
}

#[crate::worker]
impl Worker for RemoteForwarder {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> crate::Result<()> {
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

        let msg = TransportMessage {
            version: 1,
            onward_route: self.destination.clone(),
            return_route,
            payload,
        };

        ctx.forward(msg).await?;

        Ok(())
    }
}
