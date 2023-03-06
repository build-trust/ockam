//! Registration with Ockam Hub, and forwarding to local workers.
#![deny(missing_docs)]

use crate::{Context, Message, OckamError};
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{
    Address, AllowAll, AllowSourceAddress, Any, Decodable, DenyAll, Mailbox, Mailboxes,
    OutgoingAccessControl, Result, Route, Routed, Worker,
};
use ockam_node::{DelayedEvent, WorkerBuilder};
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

/// This Worker is responsible for registering on Ockam Hub and forwarding messages to local Worker
pub struct RemoteForwarder {
    /// Address used from other node
    main_address: Address,
    /// Address used for heartbeat messages
    heartbeat_address: Address,
    registration_route: Route,
    registration_payload: String,
    callback_address: Option<Address>,
    // We only use Heartbeat for static RemoteForwarder
    heartbeat: Option<DelayedEvent<Vec<u8>>>,
    heartbeat_interval: Duration,
}

impl RemoteForwarder {
    fn new(
        main_address: Address,
        heartbeat_address: Address,
        registration_route: Route,
        registration_payload: String,
        callback_address: Address,
        heartbeat: Option<DelayedEvent<Vec<u8>>>,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            main_address,
            heartbeat_address,
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
        outgoing_access_control: impl OutgoingAccessControl,
    ) -> Result<RemoteForwarderInfo> {
        let main_address = Address::random_tagged("RemoteForwarder.static.main");
        let heartbeat_address = Address::random_tagged("RemoteForwarder.static.heartbeat");

        let address = Address::random_tagged("RemoteForwarder.static.child");
        let mut child_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                address,
                Arc::new(AllowSourceAddress(main_address.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = hub_route
            .into()
            .modify()
            .append("static_forwarding_service")
            .into();

        let heartbeat = DelayedEvent::create(ctx, heartbeat_address.clone(), vec![]).await?;
        let heartbeat_source_address = heartbeat.address();
        let forwarder = Self::new(
            main_address.clone(),
            heartbeat_address.clone(),
            registration_route,
            alias.into(),
            child_ctx.address(),
            Some(heartbeat),
            Duration::from_secs(5),
        );

        debug!("Starting static RemoteForwarder at {}", &heartbeat_address);

        let mailboxes = Mailboxes::new(
            Mailbox::new(
                main_address,
                Arc::new(AllowAll), // Messages should have the same return_route, we check for that in `handle_message`
                Arc::new(outgoing_access_control),
            ),
            vec![Mailbox::new(
                heartbeat_address,
                Arc::new(AllowSourceAddress(heartbeat_source_address)),
                Arc::new(DenyAll),
            )],
        );
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

        let resp = child_ctx
            .receive::<RemoteForwarderInfo>()
            .await?
            .take()
            .body();

        Ok(resp)
    }

    /// Create and start new ephemeral RemoteForwarder at random address with given Ockam Hub route
    pub async fn create(
        ctx: &Context,
        hub_route: impl Into<Route>,
        outgoing_access_control: impl OutgoingAccessControl,
    ) -> Result<RemoteForwarderInfo> {
        let main_address = Address::random_tagged("RemoteForwarder.ephemeral.main");
        let heartbeat_address = Address::random_tagged("RemoteForwarder.ephemeral.heartbeat");
        let address = Address::random_tagged("RemoteForwarder.ephemeral.child");

        let mut child_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                address,
                Arc::new(AllowSourceAddress(main_address.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = hub_route
            .into()
            .modify()
            .append("forwarding_service")
            .into();

        let forwarder = Self::new(
            main_address.clone(),
            heartbeat_address.clone(),
            registration_route,
            "register".to_string(),
            child_ctx.address(),
            None,
            Duration::from_secs(10),
        );

        debug!("Starting ephemeral RemoteForwarder at {}", &main_address);
        // FIXME: @ac
        let mailboxes = Mailboxes::main(
            main_address,
            Arc::new(AllowAll), // Messages should have the same return_route, we check for that in `handle_message`
            Arc::new(outgoing_access_control),
        );
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

        let resp = child_ctx
            .receive::<RemoteForwarderInfo>()
            .await?
            .take()
            .body();

        Ok(resp)
    }

    /// Create and start new static RemoteForwarder without heart beats
    // This is a temporary kind of RemoteForwarder that will only run on
    // rust nodes (hence the `forwarding_service` addr to create static forwarders).
    // We will use it while we don't have heartbeats implemented on rust nodes.
    pub async fn create_static_without_heartbeats(
        ctx: &Context,
        hub_route: impl Into<Route>,
        alias: impl Into<String>,
        outgoing_access_control: impl OutgoingAccessControl,
    ) -> Result<RemoteForwarderInfo> {
        let main_address = Address::random_tagged("RemoteForwarder.static_w/o_heartbeats.main");
        let heartbeat_address =
            Address::random_tagged("RemoteForwarder.static_w/o_heartbeats.heartbeat");
        let address = Address::random_tagged("RemoteForwarder.static_w/o_heartbeats.child");
        let mut child_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                address,
                Arc::new(AllowSourceAddress(main_address.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = hub_route
            .into()
            .modify()
            .append("forwarding_service")
            .into();

        let forwarder = Self::new(
            main_address.clone(),
            heartbeat_address.clone(),
            registration_route,
            alias.into(),
            child_ctx.address(),
            None,
            Duration::from_secs(10),
        );

        debug!(
            "Starting static RemoteForwarder without heartbeats at {}",
            &main_address
        );
        // FIXME: @ac
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                main_address,
                Arc::new(AllowAll), // Messages should have the same return_route, we check for that in `handle_message`
                Arc::new(outgoing_access_control),
            ),
            vec![],
        );
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

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
            self.main_address.clone(),
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
        if msg.msg_addr() == self.heartbeat_address {
            ctx.send_from_address(
                self.registration_route.clone(),
                self.registration_payload.clone(),
                self.main_address.clone(),
            )
            .await?;

            if let Some(heartbeat) = &mut self.heartbeat {
                heartbeat.schedule(self.heartbeat_interval).await?;
            }

            return Ok(());
        }

        // FIXME: @ac check that return address is the same
        // We are the final recipient of the message because it's registration response for our Worker
        if msg.onward_route().recipient()? == self.main_address {
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
                let address = match route.clone().recipient()?.to_string().strip_prefix("0#") {
                    Some(addr) => addr.to_string(),
                    None => return Err(OckamError::InvalidHubResponse.into()),
                };

                ctx.send_from_address(
                    callback_address,
                    RemoteForwarderInfo {
                        forwarding_route: route,
                        remote_address: address,
                        worker_address: ctx.address(),
                    },
                    self.main_address.clone(),
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
            ctx.forward_from_address(message, self.main_address.clone())
                .await?;

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
    use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpTransport};
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

        ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
            .await?;

        let tcp = TcpTransport::create(ctx).await?;
        let node_in_hub = tcp
            .connect(cloud_address, TcpConnectionTrustOptions::new())
            .await?;

        let remote_info = RemoteForwarder::create(ctx, node_in_hub.clone(), AllowAll).await?;

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

        ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
            .await?;

        let tcp = TcpTransport::create(ctx).await?;

        let node_in_hub = tcp
            .connect(cloud_address, TcpConnectionTrustOptions::new())
            .await?;
        let _ = RemoteForwarder::create_static(ctx, node_in_hub.clone(), "alias", AllowAll).await?;

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
