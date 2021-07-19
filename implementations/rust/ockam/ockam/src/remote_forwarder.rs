#![deny(missing_docs)]

use crate::{Context, OckamError};
use ockam_core::lib::net::SocketAddr;
use ockam_core::{Address, Any, LocalMessage, Result, Route, Routed, TransportMessage, Worker};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};


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
    name: Option<String>
}

impl RemoteForwarder {
    fn new(hub_addr: SocketAddr, destination: Address, callback_address: Address, service_address: String, name: Option<String>) -> Self {
        let route = Self::service_route(hub_addr, service_address);
        let destination = Route::new().append(destination).into();
        Self {
            route,
            destination,
            callback_address,
            name
        }
    }

    fn service_route(hub_addr: SocketAddr, service_address: String) -> Route {
        Route::new()
            .append(format!("1#{}", hub_addr))
            .append(service_address)
            .into()
    }

    /// Create and start new RemoteForwarder with named alias
    /// Similar to 'create', but using alias_service instead
    pub async fn create_named<A: Into<Address>, S: Into<String>>(
        ctx: &Context,
        hub_addr: S,
        destination: A,
        name: S,
    ) -> Result<RemoteForwarderInfo> {
        if let Ok(hub_addr) = hub_addr.into().parse::<SocketAddr>() {
            let address: Address = random();
            let mut child_ctx = ctx.new_context(address).await?;
            let forwarder = Self::new(hub_addr, destination.into(), child_ctx.address(), "alias_service".into(), Some(name.into()));

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

    /// Delete an alias registered with create_named
    pub async fn delete_named<S: Into<String>>(
        ctx: &Context,
        hub_addr: S,
        name: S,
    ) -> Result<()> {
        if let Ok(hub_addr) = hub_addr.into().parse::<SocketAddr>() {
            let address: Address = random();
            let mut child_ctx = ctx.new_context(address).await?;

            let payload = ["DEL:".to_string(), name.into()].concat();
            child_ctx.send(Self::service_route(hub_addr, "alias_service".into()), payload).await?;
            let resp = child_ctx.receive::<String>().await?.take();

            let resp = resp.body();
            match resp.as_str() {
                "OK" => Ok(()),
                _ => Err(OckamError::InvalidHubResponse.into()),
            }
        } else {
            Err(OckamError::InvalidParameter.into())
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
            let forwarder = Self::new(hub_addr, destination.into(), child_ctx.address(), "forwarding_service".into(), None);

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

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("RemoteForwarder registering...");
        // Implied that if we don't have a name we're using the old forwarding_service
        let register_payload = match self.name.clone() {
            Some(name) => ["REG:".to_string(), name.into()].concat(),
            None => "OK".to_string()
        };
        ctx.send(self.route.clone(), register_payload).await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.return_route();
        let resp = resp.body();
        match resp.as_str() {
            "OK" => self.route = route.clone(),
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


