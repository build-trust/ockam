use ockam::identity::Identifier;
use ockam_abac::PolicyExpression;
use ockam_core::api::{Reply, Request};
use ockam_core::{async_trait, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::time::Duration;

use crate::nodes::models::portal::{CreateInlet, InletStatusView};
use crate::nodes::service::tcp_inlets::Inlets;
use crate::nodes::BackgroundNodeClient;

#[allow(clippy::too_many_arguments)]
pub fn create_inlet_payload(
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
) -> CreateInlet {
    let mut payload = CreateInlet::new(
        listen_addr.clone(),
        target_redundancy,
        outlet_addresses.clone(),
        alias.into(),
        authorized_identifier.clone(),
        wait_connection,
        enable_udp_puncture,
        disable_tcp_fallback,
        privileged,
        skip_handshake,
        enable_nagle,
    );
    if let Some(e) = policy_expression.as_ref() {
        payload.set_policy_expression(e.clone())
    }
    if let Some(identifier) = secure_channel_identifier {
        payload.set_secure_channel_identifier(identifier.clone())
    }
    if let Some(tls_provider) = tls_certificate_provider {
        payload.set_tls_certificate_provider(tls_provider.clone())
    }
    payload.set_ping_timeout(ping_timeout);
    payload.set_prefix_route(prefix_route);
    payload.set_wait_for_outlet(wait_for_outlet_timeout);
    payload
}

#[async_trait]
impl Inlets for BackgroundNodeClient {
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
    ) -> miette::Result<Reply<InletStatusView>> {
        let request = {
            let payload = create_inlet_payload(
                listen_addr,
                target_redundancy,
                outlet_addresses,
                alias,
                authorized_identifier,
                policy_expression,
                ping_timeout,
                wait_for_outlet_timeout,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
                privileged,
                tls_certificate_provider,
                skip_handshake,
                enable_nagle,
                prefix_route,
            );
            Request::post("/node/inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }

    async fn show_inlet(
        &self,
        ctx: &Context,
        alias: &str,
    ) -> miette::Result<Reply<InletStatusView>> {
        let request = Request::get(format!("/node/inlet/{alias}"));
        self.ask_and_get_reply(ctx, request).await
    }

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>> {
        let request = Request::delete(format!("/node/inlet/{inlet_alias}"));
        self.tell_and_get_reply(ctx, request).await
    }
}
