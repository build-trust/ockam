use ockam::identity::Identifier;
use ockam::Result;
use ockam_abac::PolicyExpression;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::time::Duration;

use crate::nodes::models::portal::InletStatus;
use crate::nodes::InMemoryNode;

impl InMemoryNode {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: HostnamePort,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
    ) -> Result<InletStatus> {
        self.node_manager
            .create_inlet(
                ctx,
                listen_addr.clone(),
                prefix_route.clone(),
                suffix_route.clone(),
                outlet_addr.clone(),
                alias,
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
            )
            .await
    }
}
