use ockam::identity::Identifier;
use ockam_abac::PolicyExpression;
use ockam_core::api::Reply;
use ockam_core::{async_trait, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::time::Duration;

use crate::nodes::models::portal::InletStatusView;

#[async_trait]
pub trait Inlets {
    #[allow(clippy::too_many_arguments)]
    async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: &HostnamePort,
        target_redundancy: usize,
        outlet_addresses: Vec<MultiAddr>,
        alias: &str,
        authorized_identifier: &Option<Identifier>,
        policy_expression: &Option<PolicyExpression>,
        ping_timeout: Duration,
        wait_for_outlet_timeout: Duration,
        wait_connection: bool,
        secure_channel_identifier: &Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
        privileged: bool,
        tls_certificate_provider: &Option<MultiAddr>,
        skip_handshake: bool,
        enable_nagle: bool,
        prefix_route: Route,
    ) -> miette::Result<Reply<InletStatusView>>;

    async fn show_inlet(
        &self,
        ctx: &Context,
        alias: &str,
    ) -> miette::Result<Reply<InletStatusView>>;

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>>;
}
