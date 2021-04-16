use crate::{async_worker, Context, OckamError};
use ockam_core::lib::net::SocketAddr;
use ockam_core::{Address, Any, Result, Route, Routed, TransportMessage, Worker};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ForwarderInfo {
    forwarding_route: Route,
    remote_address: String,
    worker_address: Address,
}

impl ForwarderInfo {
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

/// This Worker is responsible for registering on Ockam Hub and forwarding message with type T to
/// local Worker
pub struct Forwarder {
    route: Route,
    destination: Route,
    callback_address: Address,
}

impl Forwarder {
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

    /// Create and start new Forwarder with given Ockam Hub address
    /// and Address of destionation Worker that should receive forwarded messages
    pub async fn create<A: Into<Address>, S: Into<String>>(
        ctx: &mut Context,
        hub_addr: S,
        destination: A,
    ) -> Result<ForwarderInfo> {
        if let Ok(hub_addr) = hub_addr.into().parse::<SocketAddr>() {
            let forwarder = Self::new(hub_addr, destination.into(), ctx.address());

            let worker_address: Address = random();
            ctx.start_worker(worker_address, forwarder).await?;

            let resp = ctx.receive::<ForwarderInfo>().await?.take().body();

            Ok(resp)
        } else {
            Err(OckamError::InvalidParameter.into())
        }
    }
}

#[async_worker]
impl Worker for Forwarder {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> crate::Result<()> {
        info!("Forwarder registering...");
        ctx.send(self.route.clone(), "register".to_string()).await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.return_route();
        let resp = resp.body();
        match resp.as_str() {
            "register" => self.route = route.clone(),
            _ => return Err(OckamError::InvalidHubResponse.into()),
        }
        info!("Forwarder route: {}", route);
        let address;
        if let Some(a) = route.clone().recipient().to_string().strip_prefix("0#") {
            address = a.to_string();
        } else {
            return Err(OckamError::InvalidHubResponse.into());
        }

        ctx.send(
            self.callback_address.clone(),
            ForwarderInfo {
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
        info!("Forwarder received message");

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
