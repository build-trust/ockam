use super::NodeManagerWorker;
use crate::gateway::interceptor::HttpAuthInterceptorFactory;
use crate::gateway::token_lease_refresher::TokenLeaseRefresher;
use crate::nodes::models::influxdb_portal::{CreateInfluxDBInlet, CreateInfluxDBOutlet};
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletStatus, OutletAccessControl, OutletStatus,
};
use crate::nodes::service::tcp_inlets::create_inlet_payload;
use crate::nodes::BackgroundNodeClient;
use crate::{ApiError, DefaultAddress};
use ockam::flow_control::FlowControls;
use ockam::identity::Identifier;
use ockam::{Address, Context, Result, Route};
use ockam_abac::PolicyExpression;
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::async_trait;
use ockam_core::route;
use ockam_multiaddr::MultiAddr;
use ockam_transport_core::HostnamePort;
use ockam_transport_tcp::{PortalInletInterceptor, PortalOutletInterceptor};
use std::sync::Arc;
use std::time::Duration;

impl NodeManagerWorker {
    pub(super) async fn start_influxdb_outlet_service(
        &self,
        ctx: &Context,
        body: CreateInfluxDBOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        let CreateOutlet {
            hostname_port,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression,
            ebpf,
            tls,
        } = body.tcp_outlet;
        let interceptor_addr = self
            .node_manager
            .registry
            .outlets
            .generate_worker_addr(worker_addr)
            .await;
        let outlet_addr: Address = format!("{:}_tcp", interceptor_addr.address()).into();
        self.create_http_outlet_interceptor(
            ctx,
            interceptor_addr,
            outlet_addr.clone(),
            policy_expression.clone(),
            body.service_address,
        )
        .await
        .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;

        match self
            .node_manager
            .create_outlet(
                ctx,
                hostname_port,
                tls,
                Some(outlet_addr),
                reachable_from_default_secure_channel,
                OutletAccessControl::WithPolicyExpression(policy_expression),
                ebpf,
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn start_influxdb_inlet_service(
        &self,
        ctx: &Context,
        body: CreateInfluxDBInlet,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        let CreateInlet {
            listen_addr,
            outlet_addr,
            alias,
            authorized,
            wait_for_outlet_duration,
            policy_expression,
            wait_connection,
            secure_channel_identifier,
            enable_udp_puncture,
            disable_tcp_fallback,
            ebpf,
            tls_certificate_provider,
            suffix_route,
        } = body.tcp_inlet.clone();
        let interceptor_addr = self
            .create_http_auth_interceptor(
                ctx,
                &alias,
                policy_expression.clone(),
                body.service_address.clone(),
            )
            .await
            .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                route![interceptor_addr],
                suffix_route,
                outlet_addr,
                alias,
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
                ebpf,
                tls_certificate_provider,
            )
            .await
        {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    async fn create_http_outlet_interceptor(
        &self,
        ctx: &Context,
        interceptor_address: Address,
        outlet_address: Address,
        outlet_policy_expression: Option<PolicyExpression>,
        route_to_lessor: MultiAddr,
    ) -> Result<(), Error> {
        let default_secure_channel_listener_flow_control_id = ctx
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;

        let policy_access_control = self
            .node_manager
            .policy_access_control(
                self.node_manager.project_authority().clone(),
                Resource::new(outlet_address.to_string(), ResourceType::TcpOutlet),
                Action::HandleMessage,
                outlet_policy_expression.clone(),
            )
            .await?;

        let spawner_flow_control_id = FlowControls::generate_flow_control_id();

        let token_refresher =
            TokenLeaseRefresher::new(ctx, self.node_manager.clone(), route_to_lessor).await?;
        let http_interceptor_factory = Arc::new(HttpAuthInterceptorFactory::new(token_refresher));

        PortalOutletInterceptor::create(
            ctx,
            interceptor_address.clone(),
            Some(spawner_flow_control_id.clone()),
            http_interceptor_factory,
            Arc::new(policy_access_control.create_outgoing(ctx).await?),
            Arc::new(policy_access_control.create_incoming()),
        )
        .await?;

        // every secure channel can reach this service
        let flow_controls = ctx.flow_controls();
        flow_controls.add_consumer(
            interceptor_address.clone(),
            &default_secure_channel_listener_flow_control_id,
        );

        // this spawner flow control id is used to control communication with dynamically created
        // outlets
        flow_controls.add_spawner(interceptor_address, &spawner_flow_control_id);

        // allow communication with the kafka bootstrap outlet
        flow_controls.add_consumer(outlet_address, &spawner_flow_control_id);
        Ok(())
    }

    async fn create_http_auth_interceptor(
        &self,
        ctx: &Context,
        inlet_alias: &String,
        inlet_policy_expression: Option<PolicyExpression>,
        route_to_lessor: MultiAddr,
    ) -> Result<Address, Error> {
        let interceptor_address: Address = (inlet_alias.to_owned() + "_http_interceptor").into();
        let policy_access_control = self
            .node_manager
            .policy_access_control(
                self.node_manager.project_authority().clone(),
                Resource::new(interceptor_address.to_string(), ResourceType::TcpInlet),
                Action::HandleMessage,
                inlet_policy_expression,
            )
            .await?;

        let token_refresher =
            TokenLeaseRefresher::new(ctx, self.node_manager.clone(), route_to_lessor).await?;
        let http_interceptor_factory = Arc::new(HttpAuthInterceptorFactory::new(token_refresher));

        PortalInletInterceptor::create(
            ctx,
            interceptor_address.clone(),
            http_interceptor_factory,
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(ctx).await?),
        )
        .await?;
        Ok(interceptor_address)
    }
}

#[async_trait]
pub trait InfluxDBPortals {
    #[allow(clippy::too_many_arguments)]
    async fn create_influxdb_inlet(
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
        tls_certificate_provider: &Option<MultiAddr>,
        suffix_route: Route,
        token_leaser: MultiAddr,
    ) -> miette::Result<Reply<InletStatus>>;

    async fn create_influxdb_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        token_leaser: MultiAddr,
    ) -> miette::Result<OutletStatus>;
}

#[async_trait]
impl InfluxDBPortals for BackgroundNodeClient {
    #[instrument(skip(self, ctx))]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    async fn create_influxdb_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        token_leaser: MultiAddr,
    ) -> miette::Result<OutletStatus> {
        let mut outlet_payload = CreateOutlet::new(to, tls, from.cloned(), true, false);
        if let Some(policy_expression) = policy_expression {
            outlet_payload.set_policy_expression(policy_expression);
        }
        let payload = CreateInfluxDBOutlet::new(outlet_payload, token_leaser);
        let req = Request::post("/node/influxdb_outlet").body(payload);
        //TODO: difference between ask and ask_and_get_reply?
        let result: OutletStatus = self.ask(ctx, req).await?;
        Ok(result)
    }

    async fn create_influxdb_inlet(
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
        tls_certificate_provider: &Option<MultiAddr>,
        suffix_route: Route,
        token_leaser: MultiAddr,
    ) -> miette::Result<Reply<InletStatus>> {
        let request = {
            let inlet_payload = create_inlet_payload(
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
                false,
                tls_certificate_provider,
                suffix_route,
            );
            let payload = CreateInfluxDBInlet::new(inlet_payload, token_leaser);
            Request::post("/node/influxdb_inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }
}
