//! Registration with Ockam Hub, and forwarding to local workers.
#![deny(missing_docs)]

use crate::{Context, Message, OckamError};
use core::time::Duration;
use ockam_core::compat::rand::random;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Address, AddressSet, Any, Decodable, Result, Route, Routed, Worker};
use ockam_node::DelayedEvent;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Information about a remotely forwarded worker.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Message)]
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

/// All addresses `RemoteForwarder` is registered for
#[derive(Clone)]
struct Addresses {
    /// Address used from other node
    main_address: Address,
    /// Address used for heartbeat messages
    heartbeat_address: Address,
}

impl Distribution<Addresses> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Addresses {
        Addresses {
            main_address: rng.gen(),
            heartbeat_address: rng.gen(),
        }
    }
}

impl Addresses {
    fn into_set(self) -> AddressSet {
        vec![self.main_address, self.heartbeat_address].into()
    }
}

/// This Worker is responsible for registering on Ockam Hub and forwarding messages to local Worker
pub struct RemoteForwarder {
    addresses: Addresses,
    registration_route: Route,
    registration_payload: String,
    callback_address: Option<Address>,
    // We only use Heartbeat for static RemoteForwarder
    heartbeat: Option<DelayedEvent<Vec<u8>>>,
    heartbeat_interval: Duration,
}

impl RemoteForwarder {
    fn new(
        addresses: Addresses,
        registration_route: Route,
        registration_payload: String,
        callback_address: Address,
        heartbeat: Option<DelayedEvent<Vec<u8>>>,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            addresses,
            registration_route,
            registration_payload,
            callback_address: Some(callback_address),
            heartbeat,
            heartbeat_interval,
        }
    }

    /// Create and start static RemoteForwarder at predefined address with given Ockam Hub route
    pub async fn create_static(
        ctx: &Context,
        hub_route: impl Into<Route>,
        alias: impl Into<String>,
    ) -> Result<RemoteForwarderInfo> {
        let address: Address = random();
        let mut child_ctx = ctx.new_detached(address).await?;

        let addresses: Addresses = random();

        let registration_route = hub_route
            .into()
            .modify()
            .append("static_forwarding_service")
            .into();

        let heartbeat =
            DelayedEvent::create(ctx, addresses.heartbeat_address.clone(), vec![]).await?;
        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            alias.into(),
            child_ctx.address(),
            Some(heartbeat),
            Duration::from_secs(5),
        );

        debug!(
            "Starting static RemoteForwarder at {}",
            &addresses.heartbeat_address
        );
        ctx.start_worker(addresses.into_set(), forwarder).await?;

        let resp = child_ctx
            .receive::<RemoteForwarderInfo>()
            .await?
            .take()
            .body();

        Ok(resp)
    }

    /// Create and start new ephemeral RemoteForwarder at random address with given Ockam Hub route
    pub async fn create(ctx: &Context, hub_route: impl Into<Route>) -> Result<RemoteForwarderInfo> {
        let address: Address = random();
        let mut child_ctx = ctx.new_detached(address).await?;

        let addresses: Addresses = random();

        let registration_route = hub_route
            .into()
            .modify()
            .append("forwarding_service")
            .into();

        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            "register".to_string(),
            child_ctx.address(),
            None,
            Duration::from_secs(10),
        );

        debug!(
            "Starting ephemeral RemoteForwarder at {}",
            &addresses.main_address
        );
        ctx.start_worker(addresses.main_address, forwarder).await?;

        let resp = child_ctx
            .receive::<RemoteForwarderInfo>()
            .await?
            .take()
            .body();

        Ok(resp)
    }

    /// Create and start new static RemoteForwarder without heart beats
    pub async fn create_static_without_heartbeats(
        ctx: &Context,
        hub_route: impl Into<Route>,
        alias: impl Into<String>,
    ) -> Result<RemoteForwarderInfo> {
        let address: Address = random();
        let mut child_ctx = ctx.new_detached(address).await?;

        let addresses: Addresses = random();

        let registration_route = hub_route
            .into()
            .modify()
            .append("forwarding_service")
            .into();

        // let remote_address = Address::random_local().without_type().to_string();
        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            alias.into(),
            child_ctx.address(),
            None,
            Duration::from_secs(10),
        );

        debug!(
            "Starting ephemeral RemoteForwarder at {}",
            &addresses.main_address
        );
        ctx.start_worker(addresses.main_address, forwarder).await?;

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
        debug!("RemoteForwarder registration...");

        ctx.send_from_address(
            self.registration_route.clone(),
            self.registration_payload.clone(),
            self.addresses.main_address.clone(),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Heartbeat message, send registration message
        if msg.msg_addr() == self.addresses.heartbeat_address {
            ctx.send_from_address(
                self.registration_route.clone(),
                self.registration_payload.clone(),
                self.addresses.main_address.clone(),
            )
            .await?;

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }

            return Ok(());
        }

        // We are the final recipient of the message because it's registration response for our Worker
        if msg.onward_route().recipient() == self.addresses.main_address {
            debug!("RemoteForwarder received service message");

            let payload =
                Vec::<u8>::decode(msg.payload()).map_err(|_| OckamError::InvalidHubResponse)?;
            let payload = String::from_utf8(payload).map_err(|_| OckamError::InvalidHubResponse)?;
            if payload != self.registration_payload {
                return Err(OckamError::InvalidHubResponse.into());
            }

            if let Some(callback_address) = self.callback_address.take() {
                let route = msg.return_route();

                info!("RemoteForwarder registered with route: {}", route);
                let address = match route.clone().recipient().to_string().strip_prefix("0#") {
                    Some(addr) => addr.to_string(),
                    None => return Err(OckamError::InvalidHubResponse.into()),
                };

                ctx.send(
                    callback_address,
                    RemoteForwarderInfo {
                        forwarding_route: route,
                        remote_address: address,
                        worker_address: ctx.address(),
                    },
                )
                .await?;
            }

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }
        } else {
            debug!("RemoteForwarder received payload message");

            let mut message = msg.into_local_message();
            let transport_message = message.transport_mut();

            // Remove my address from the onward_route
            transport_message.onward_route.step()?;

            // Send the message on its onward_route
            ctx.forward(message).await?;

            // We received message from the other node, our registration is still alive, let's reset
            // heartbeat timer
            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::workers::Echoer;
    use ockam_core::route;
    use ockam_transport_tcp::{TcpTransport, TCP};
    use std::env;

    fn get_cloud_address() -> Option<String> {
        if let Ok(v) = env::var("CLOUD_ADDRESS") {
            if !v.is_empty() {
                return Some(v);
            }
        }

        warn!("No CLOUD_ADDRESS specified, skipping the test");

        None
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn forwarding__ephemeral_address__should_respond(ctx: &mut Context) -> Result<()> {
        let cloud_address = if let Some(c) = get_cloud_address() {
            c
        } else {
            ctx.stop().await?;
            return Ok(());
        };

        ctx.start_worker("echoer", Echoer).await?;

        TcpTransport::create(ctx).await?;

        let node_in_hub = (TCP, cloud_address);
        let remote_info = RemoteForwarder::create(ctx, node_in_hub.clone()).await?;

        let resp = ctx
            .send_and_receive::<_, _, String>(
                route![node_in_hub, remote_info.remote_address(), "echoer"],
                "Hello".to_string(),
            )
            .await?;

        assert_eq!(resp, "Hello");

        ctx.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn forwarding__static_address__should_respond(ctx: &mut Context) -> Result<()> {
        let cloud_address = if let Some(c) = get_cloud_address() {
            c
        } else {
            ctx.stop().await?;
            return Ok(());
        };

        ctx.start_worker("echoer", Echoer).await?;

        TcpTransport::create(ctx).await?;

        let node_in_hub = (TCP, cloud_address);
        let _ = RemoteForwarder::create_static(ctx, node_in_hub.clone(), "alias").await?;

        let resp = ctx
            .send_and_receive::<_, _, String>(
                route![node_in_hub, "forward_to_alias", "echoer"],
                "Hello".to_string(),
            )
            .await?;

        assert_eq!(resp, "Hello");

        ctx.stop().await
    }
}
