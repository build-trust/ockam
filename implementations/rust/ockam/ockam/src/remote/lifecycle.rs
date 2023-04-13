use crate::remote::{Addresses, RemoteForwarder, RemoteForwarderInfo, RemoteForwarderOptions};
use crate::Context;
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{
    route, Address, AllowAll, AllowSourceAddress, DenyAll, Mailbox, Mailboxes,
    OutgoingAccessControl, Result, Route,
};
use ockam_node::{DelayedEvent, WorkerBuilder};
use tracing::debug;

#[derive(Clone, Copy)]
pub(super) enum ForwardType {
    Static,
    Ephemeral,
    StaticWithoutHeartbeats,
}

impl ForwardType {
    pub fn str(&self) -> &'static str {
        match self {
            ForwardType::Static => "static",
            ForwardType::Ephemeral => "ephemeral",
            ForwardType::StaticWithoutHeartbeats => "static_w/o_heartbeats",
        }
    }
}

impl RemoteForwarder {
    fn mailboxes(
        addresses: Addresses,
        heartbeat_source_address: Option<Address>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Mailboxes {
        let main_internal = Mailbox::new(
            addresses.main_internal,
            Arc::new(DenyAll),
            outgoing_access_control,
        );

        let main_remote = Mailbox::new(
            addresses.main_remote,
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        );

        let mut additional_mailboxes = vec![main_remote];

        if let Some(heartbeat_source_address) = heartbeat_source_address {
            let heartbeat = Mailbox::new(
                addresses.heartbeat,
                Arc::new(AllowSourceAddress(heartbeat_source_address)),
                Arc::new(DenyAll),
            );
            additional_mailboxes.push(heartbeat);
        }

        Mailboxes::new(main_internal, additional_mailboxes)
    }
}

impl RemoteForwarder {
    fn new(
        addresses: Addresses,
        registration_route: Route,
        registration_payload: String,
        heartbeat: Option<DelayedEvent<Vec<u8>>>,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            addresses,
            completion_msg_sent: false,
            registration_route,
            registration_payload,
            heartbeat,
            heartbeat_interval,
        }
    }

    /// Create and start static RemoteForwarder at predefined address with given Ockam Orchestrator route
    pub async fn create_static(
        ctx: &Context,
        hub_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        let addresses = Addresses::generate(ForwardType::Static);

        let mut child_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                addresses.completion_callback.clone(),
                Arc::new(AllowSourceAddress(addresses.main_remote.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = route![hub_route.into(), "static_forwarding_service"];

        let heartbeat = DelayedEvent::create(ctx, addresses.heartbeat.clone(), vec![]).await?;
        let heartbeat_source_address = heartbeat.address();

        let flow_control_id =
            options.setup_flow_control(ctx.flow_controls(), &addresses, registration_route.next()?);
        let outgoing_access_control =
            options.create_access_control(ctx.flow_controls(), flow_control_id);

        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            alias.into(),
            Some(heartbeat),
            Duration::from_secs(5),
        );

        debug!(
            "Starting static RemoteForwarder at {}",
            &addresses.heartbeat
        );
        let mailboxes = Self::mailboxes(
            addresses,
            Some(heartbeat_source_address),
            outgoing_access_control,
        );
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

        let resp = child_ctx.receive::<RemoteForwarderInfo>().await?.body();

        Ok(resp)
    }

    /// Create and start new ephemeral RemoteForwarder at random address with given Ockam Hub route
    pub async fn create(
        ctx: &Context,
        hub_route: impl Into<Route>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        let addresses = Addresses::generate(ForwardType::Ephemeral);

        let mut callback_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                addresses.completion_callback.clone(),
                Arc::new(AllowSourceAddress(addresses.main_remote.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = route![hub_route, "forwarding_service"];

        let flow_control_id =
            options.setup_flow_control(ctx.flow_controls(), &addresses, registration_route.next()?);
        let outgoing_access_control =
            options.create_access_control(ctx.flow_controls(), flow_control_id);

        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            "register".to_string(),
            None,
            Duration::from_secs(10),
        );

        debug!(
            "Starting ephemeral RemoteForwarder at {}",
            &addresses.main_internal
        );
        let mailboxes = Self::mailboxes(addresses, None, outgoing_access_control);
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

        let resp = callback_ctx.receive::<RemoteForwarderInfo>().await?.body();

        Ok(resp)
    }

    /// Create and start new static RemoteForwarder without heart beats
    /// This is a temporary kind of RemoteForwarder that will only run on
    /// rust nodes (hence the `forwarding_service` addr to create static forwarders).
    /// We will use it while we don't have heartbeats implemented on rust nodes.
    pub async fn create_static_without_heartbeats(
        ctx: &Context,
        hub_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteForwarderOptions,
    ) -> Result<RemoteForwarderInfo> {
        let addresses = Addresses::generate(ForwardType::StaticWithoutHeartbeats);

        let mut callback_ctx = ctx
            .new_detached_with_mailboxes(Mailboxes::main(
                addresses.completion_callback.clone(),
                Arc::new(AllowSourceAddress(addresses.main_remote.clone())),
                Arc::new(DenyAll),
            ))
            .await?;

        let registration_route = route![hub_route.into(), "forwarding_service"];

        let flow_control_id =
            options.setup_flow_control(ctx.flow_controls(), &addresses, registration_route.next()?);
        let outgoing_access_control =
            options.create_access_control(ctx.flow_controls(), flow_control_id);

        let forwarder = Self::new(
            addresses.clone(),
            registration_route,
            alias.into(),
            None,
            Duration::from_secs(10),
        );

        debug!(
            "Starting static RemoteForwarder without heartbeats at {}",
            &addresses.main_internal
        );
        let mailboxes = Self::mailboxes(addresses, None, outgoing_access_control);
        WorkerBuilder::with_mailboxes(mailboxes, forwarder)
            .start(ctx)
            .await?;

        let resp = callback_ctx.receive::<RemoteForwarderInfo>().await?.body();

        Ok(resp)
    }
}
