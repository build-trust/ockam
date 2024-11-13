use crate::remote::{Addresses, RemoteRelay, RemoteRelayInfo, RemoteRelayOptions};
use crate::Context;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    AllowAll, AllowSourceAddress, DenyAll, Mailbox, Mailboxes, OutgoingAccessControl, Result, Route,
};
use ockam_node::WorkerBuilder;
use tracing::debug;

#[derive(Clone, Copy)]
pub(super) enum RelayType {
    Static,
    Ephemeral,
}

impl RelayType {
    pub fn str(&self) -> &'static str {
        match self {
            RelayType::Static => "static",
            RelayType::Ephemeral => "ephemeral",
        }
    }
}

impl RemoteRelay {
    fn mailboxes(
        addresses: Addresses,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Mailboxes {
        let main_internal = Mailbox::new(
            addresses.main_internal,
            None,
            Arc::new(DenyAll),
            outgoing_access_control,
        );

        let main_remote = Mailbox::new(
            addresses.main_remote,
            None,
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        );

        Mailboxes::new(main_internal, vec![main_remote])
    }
}

impl RemoteRelay {
    fn new(
        addresses: Addresses,
        registration_route: Route,
        registration_payload: String,
        flow_control_id: Option<FlowControlId>,
    ) -> Self {
        Self {
            addresses,
            completion_msg_sent: false,
            registration_route,
            registration_payload,
            flow_control_id,
        }
    }

    /// Create and start static RemoteRelay at predefined address with given Ockam Orchestrator route
    pub async fn create_static(
        ctx: &Context,
        orchestrator_route: impl Into<Route>,
        alias: impl Into<String>,
        options: RemoteRelayOptions,
    ) -> Result<RemoteRelayInfo> {
        let addresses = Addresses::generate(RelayType::Static);

        let mut callback_ctx = ctx.new_detached_with_mailboxes(Mailboxes::primary(
            addresses.completion_callback.clone(),
            Arc::new(AllowSourceAddress(addresses.main_remote.clone())),
            Arc::new(DenyAll),
        ))?;

        let registration_route = orchestrator_route.into() + "static_forwarding_service";

        let flow_control_id =
            options.setup_flow_control(ctx.flow_controls(), &addresses, registration_route.next()?);
        let outgoing_access_control =
            options.create_access_control(ctx.flow_controls(), flow_control_id.clone());

        let relay = Self::new(
            addresses.clone(),
            registration_route,
            alias.into(),
            flow_control_id,
        );

        debug!("Starting static RemoteRelay at {}", &addresses.main_remote);
        let mailboxes = Self::mailboxes(addresses, outgoing_access_control);
        WorkerBuilder::new(relay)
            .with_mailboxes(mailboxes)
            .start(ctx)?;

        let resp = callback_ctx
            .receive::<RemoteRelayInfo>()
            .await?
            .into_body()?;

        Ok(resp)
    }

    /// Create and start new ephemeral RemoteRelay at random address with given Ockam Orchestrator route
    pub async fn create(
        ctx: &Context,
        orchestrator_route: impl Into<Route>,
        options: RemoteRelayOptions,
    ) -> Result<RemoteRelayInfo> {
        let addresses = Addresses::generate(RelayType::Ephemeral);

        let mut callback_ctx = ctx.new_detached_with_mailboxes(Mailboxes::primary(
            addresses.completion_callback.clone(),
            Arc::new(AllowSourceAddress(addresses.main_remote.clone())),
            Arc::new(DenyAll),
        ))?;

        let registration_route = orchestrator_route.into() + "forwarding_service";

        let flow_control_id =
            options.setup_flow_control(ctx.flow_controls(), &addresses, registration_route.next()?);
        let outgoing_access_control =
            options.create_access_control(ctx.flow_controls(), flow_control_id.clone());

        let relay = Self::new(
            addresses.clone(),
            registration_route,
            "register".to_string(),
            flow_control_id,
        );

        debug!(
            "Starting ephemeral RemoteRelay at {}",
            &addresses.main_internal
        );
        let mailboxes = Self::mailboxes(addresses, outgoing_access_control);
        WorkerBuilder::new(relay)
            .with_mailboxes(mailboxes)
            .start(ctx)?;

        let resp = callback_ctx
            .receive::<RemoteRelayInfo>()
            .await?
            .into_body()?;

        Ok(resp)
    }
}
