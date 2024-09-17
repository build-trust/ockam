use ockam::identity::Identifier;
use ockam_abac::PolicyExpression;
use ockam_core::api::{Reply, Request};
use ockam_core::async_trait;
use ockam_multiaddr::proto::Project as ProjectProto;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::time::Duration;

use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::service::tcp_inlets::Inlets;
use crate::nodes::BackgroundNodeClient;

#[allow(clippy::too_many_arguments)]
pub fn create_inlet_payload(
    listen_addr: &HostnamePort,
    outlet_addr: &MultiAddr,
    alias: &str,
    authorized_identifier: &Option<Identifier>,
    policy_expression: &Option<PolicyExpression>,
    wait_for_outlet_timeout: Duration,
    wait_connection: bool,
    secure_channel_identifier: &Option<Identifier>,
    enable_udp_puncture: bool,
    disable_tcp_fallback: bool,
    ebpf: bool,
    tls_certificate_provider: &Option<MultiAddr>,
) -> CreateInlet {
    let via_project = outlet_addr.matches(0, &[ProjectProto::CODE.into()]);
    let mut payload = if via_project {
        CreateInlet::via_project(
            listen_addr.clone(),
            outlet_addr.clone(),
            alias.into(),
            wait_connection,
            enable_udp_puncture,
            disable_tcp_fallback,
            ebpf,
        )
    } else {
        CreateInlet::to_node(
            listen_addr.clone(),
            outlet_addr.clone(),
            alias.into(),
            authorized_identifier.clone(),
            wait_connection,
            enable_udp_puncture,
            disable_tcp_fallback,
            ebpf,
        )
    };
    if let Some(e) = policy_expression.as_ref() {
        payload.set_policy_expression(e.clone())
    }
    if let Some(identifier) = secure_channel_identifier {
        payload.set_secure_channel_identifier(identifier.clone())
    }
    if let Some(tls_provider) = tls_certificate_provider {
        payload.set_tls_certificate_provider(tls_provider.clone())
    }
    payload.set_wait_ms(wait_for_outlet_timeout.as_millis() as u64);
    payload
}

#[async_trait]
impl Inlets for BackgroundNodeClient {
    async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: &HostnamePort,
        outlet_addr: &MultiAddr,
        alias: &str,
        authorized_identifier: &Option<Identifier>,
        policy_expression: &Option<PolicyExpression>,
        wait_for_outlet_timeout: Duration,
        wait_connection: bool,
        secure_channel_identifier: &Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
        ebpf: bool,
        tls_certificate_provider: &Option<MultiAddr>,
    ) -> miette::Result<Reply<InletStatus>> {
        let request = {
            let payload = create_inlet_payload(
                listen_addr,
                outlet_addr,
                alias,
                authorized_identifier,
                policy_expression,
                wait_for_outlet_timeout,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
                ebpf,
                tls_certificate_provider,
            );
            Request::post("/node/inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }

    async fn show_inlet(&self, ctx: &Context, alias: &str) -> miette::Result<Reply<InletStatus>> {
        let request = Request::get(format!("/node/inlet/{alias}"));
        self.ask_and_get_reply(ctx, request).await
    }

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>> {
        let request = Request::delete(format!("/node/inlet/{inlet_alias}"));
        self.tell_and_get_reply(ctx, request).await
    }
}
