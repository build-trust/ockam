#![deny(missing_docs)]

use crate::{Context, OckamError};
use ockam_core::{Address, Any, LocalMessage, Result, Route, Routed, TransportMessage, Worker};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Information about a remotely traced worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteTracerInfo {
    tracing_route: Route,
    remote_address: String,
    worker_address: Address,
}

impl RemoteTracerInfo {
    /// Returns the tracing route.
    pub fn tracing_route(&self) -> &Route {
        &self.tracing_route
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

/// This Worker is responsible for registering on Ockam Hub and tracing messages to local Worker
pub struct RemoteTracer {
    route: Route,
    watcher: Route,
    callback_address: Address,
}

impl RemoteTracer {
    fn new(hub_route: Route, watcher: Address, callback_address: Address) -> Self {
        let route: Route = hub_route.clone().modify().append("tracing_service").into();
        let watcher = Route::new().append(watcher).into();
        Self {
            route,
            watcher,
            callback_address,
        }
    }

    /// Create and start new RemoteTracer with given Ockam Hub address
    /// and Address of the Worker which should receive traced messages
    pub async fn create<A: Into<Address>, R: Into<Route>>(
        ctx: &Context,
        hub_route: R,
        watcher: A,
    ) -> Result<RemoteTracerInfo> {
        let address: Address = random();
        let mut child_ctx = ctx.new_context(address).await?;
        let tracer = Self::new(hub_route.into(), watcher.into(), child_ctx.address());

        let worker_address: Address = random();
        debug!("Starting RemoteTracer at {}", &worker_address);
        ctx.start_worker(worker_address, tracer).await?;

        let resp = child_ctx.receive::<RemoteTracerInfo>().await?.take().body();

        Ok(resp)
    }
}

#[crate::worker]
impl Worker for RemoteTracer {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("RemoteTracer registering...");
        ctx.send(self.route.clone(), "register".to_string()).await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.return_route();
        let resp = resp.body();
        match resp.as_str() {
            "register" => self.route = route.clone(),
            _ => return Err(OckamError::InvalidHubResponse.into()),
        }
        info!("RemoteTracer registered with route: {}", route);
        let address;
        if let Some(a) = route.clone().recipient().to_string().strip_prefix("0#") {
            address = a.to_string();
        } else {
            return Err(OckamError::InvalidHubResponse.into());
        }

        ctx.send(
            self.callback_address.clone(),
            RemoteTracerInfo {
                tracing_route: route,
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
        debug!("RemoteTracer received message");

        let msg = TransportMessage::v1(self.watcher.clone(), return_route, payload);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(())
    }
}
