use ockam::identity::Identifier;
use ockam::Result;
use ockam_abac::PolicyExpression;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::time::Duration;
use tracing::Level;

use crate::nodes::models::portal::InletStatusView;
use crate::nodes::InMemoryNode;

impl InMemoryNode {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, level = Level::TRACE)]
    pub async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: HostnamePort,
        prefix_route: Route,
        suffix_route: Route,
        target_redundancy: usize,
        outlet_addresses: Vec<MultiAddr>,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        ping_timeout: Option<Duration>,
        wait_for_outlet: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
        privileged: bool,
        tls_certificate_provider: Option<MultiAddr>,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> Result<InletStatusView> {
        self.node_manager
            .create_inlet(
                ctx,
                listen_addr,
                prefix_route,
                suffix_route,
                target_redundancy,
                outlet_addresses,
                alias,
                policy_expression,
                ping_timeout,
                wait_for_outlet,
                authorized,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
                privileged,
                tls_certificate_provider,
                skip_handshake,
                enable_nagle,
            )
            .await
    }
}
