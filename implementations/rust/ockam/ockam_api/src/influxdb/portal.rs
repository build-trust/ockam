use crate::influxdb::gateway::interceptor::HttpAuthInterceptorFactory;
use crate::influxdb::gateway::token_lease_refresher::TokenLeaseRefresher;
use crate::influxdb::{LeaseUsage, StartInfluxDBLeaseIssuerRequest};
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletStatus, OutletAccessControl, OutletStatus,
};
use crate::nodes::service::tcp_inlets::create_inlet_payload;
use crate::nodes::{BackgroundNodeClient, NodeManagerWorker};
use crate::{ApiError, DefaultAddress};
use minicbor::{CborLen, Decode, Encode};
use ockam::flow_control::FlowControls;
use ockam::identity::Identifier;
use ockam::{Address, Context, Result};
use ockam_abac::PolicyExpression;
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::async_trait;
use ockam_core::route;
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;
use ockam_transport_core::HostnamePort;
use ockam_transport_tcp::{
    read_portal_payload_length, PortalInletInterceptor, PortalOutletInterceptor,
};
use std::sync::Arc;
use std::time::Duration;

impl NodeManagerWorker {
    pub(crate) async fn start_influxdb_outlet_service(
        &self,
        ctx: &Context,
        body: CreateInfluxDBOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        debug!("Starting InfluxDB Outlet service");
        let CreateOutlet {
            hostname_port,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression,
            privileged,
            tls,
        } = body.tcp_outlet;
        let address = self
            .node_manager
            .registry
            .outlets
            .generate_worker_addr(worker_addr);
        let outlet_address = match body.influxdb_config {
            InfluxDBOutletConfig::OutletWithFixedToken(token) => {
                let outlet_addr: Address = format!("{}_outlet", address.address()).into();
                // Start the interceptor
                self.create_http_outlet_interceptor(
                    ctx,
                    address.clone(),
                    outlet_addr.clone(),
                    policy_expression.clone(),
                    token,
                )
                .await
                .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;
                outlet_addr
            }
            InfluxDBOutletConfig::StartLeaseManager(lease_manager_config) => {
                // Get the address of the lease issuer service
                let lease_issuer_address: Address =
                    format!("{}_{}", address.address(), DefaultAddress::LEASE_MANAGER).into();
                debug!(%address, ?policy_expression, "Using params");
                // Start the lease issuer service
                let req = StartInfluxDBLeaseIssuerRequest {
                    influxdb_address: hostname_port
                        .clone()
                        .into_url(if tls { "https" } else { "http" })?
                        .to_string(),
                    influxdb_org_id: lease_manager_config.influxdb_org_id,
                    influxdb_token: lease_manager_config.influxdb_token,
                    lease_permissions: lease_manager_config.lease_permissions,
                    expires_in: lease_manager_config.expires_in,
                    policy_expression: policy_expression.clone(),
                };
                self.node_manager
                    .start_influxdb_lease_issuer_service(ctx, lease_issuer_address.clone(), req)
                    .await
                    .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;
                address
            }
        };
        // Start the outlet
        match self
            .node_manager
            .create_outlet(
                ctx,
                hostname_port,
                tls,
                Some(outlet_address),
                reachable_from_default_secure_channel,
                OutletAccessControl::WithPolicyExpression(policy_expression),
                privileged,
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(crate) async fn start_influxdb_inlet_service(
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
            privileged,
            tls_certificate_provider,
        } = body.tcp_inlet.clone();

        //TODO: should be an easier way to tweak the multiaddr
        let mut issuer_route = outlet_addr.clone();
        let outlet_addr_last_service = issuer_route
            .pop_back()
            .ok_or_else(|| Response::bad_request_no_request("The outlet address is invalid"))?;
        let outlet_addr_last_service = outlet_addr_last_service
            .cast::<Service>()
            .ok_or_else(|| Response::bad_request_no_request("The outlet address is invalid"))?;

        let lease_issuer_route = if let Some(s) = body.lease_issuer_address {
            s
        } else {
            // If outlet_addr = /A/B/C, then issuer is derived as /A/B/C_lease_issuer
            let lease_issuer_service = format!(
                "{}_{}",
                &*outlet_addr_last_service,
                DefaultAddress::LEASE_MANAGER
            );
            issuer_route
                .push_back(Service::new(lease_issuer_service))
                .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;
            issuer_route
        };

        let (prefix_route, suffix_route) = match body.lease_usage {
            LeaseUsage::PerClient => {
                // Start an interceptor pointing to the lease issuer service
                let interceptor_addr = self
                    .create_http_auth_interceptor(
                        ctx,
                        &alias,
                        policy_expression.clone(),
                        lease_issuer_route,
                    )
                    .await
                    .map_err(|e| Response::bad_request_no_request(&format!("{e:?}")))?;
                (route![interceptor_addr], route![])
            }
            LeaseUsage::Shared => {
                // Http interception is done on the outlet side.
                // The suffix route is derived from the given outlet addr `/A/B/C ⇒ /A/B/C_outlet`
                (
                    route![],
                    route![format!("{}_outlet", &*outlet_addr_last_service)],
                )
            }
        };

        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                prefix_route,
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
                privileged,
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
        token_to_use: String,
    ) -> Result<(), Error> {
        debug!(%interceptor_address, %outlet_address, ?outlet_policy_expression, %token_to_use, "Creating http outlet interceptor");
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

        let token_refresher = TokenLeaseRefresher::new_with_fixed_token(token_to_use);
        let http_interceptor_factory = Arc::new(HttpAuthInterceptorFactory::new(token_refresher));

        PortalOutletInterceptor::create(
            ctx,
            interceptor_address.clone(),
            Some(spawner_flow_control_id.clone()),
            http_interceptor_factory,
            Arc::new(policy_access_control.create_outgoing(ctx)?),
            Arc::new(policy_access_control.create_incoming()),
            read_portal_payload_length(),
        )?;

        // every secure channel can reach this service
        let flow_controls = ctx.flow_controls();
        flow_controls.add_consumer(
            &interceptor_address,
            &default_secure_channel_listener_flow_control_id,
        );

        // this spawner flow control id is used to control communication with dynamically created
        // outlets
        flow_controls.add_spawner(&interceptor_address, &spawner_flow_control_id);

        // allow communication with the tcp outlet
        flow_controls.add_consumer(&outlet_address, &spawner_flow_control_id);
        Ok(())
    }

    async fn create_http_auth_interceptor(
        &self,
        ctx: &Context,
        inlet_alias: &String,
        inlet_policy_expression: Option<PolicyExpression>,
        lease_issuer_route: MultiAddr,
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
            TokenLeaseRefresher::new(ctx, Arc::downgrade(&self.node_manager), lease_issuer_route)?;
        let http_interceptor_factory = Arc::new(HttpAuthInterceptorFactory::new(token_refresher));

        PortalInletInterceptor::create(
            ctx,
            interceptor_address.clone(),
            http_interceptor_factory,
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(ctx)?),
            read_portal_payload_length(),
        )?;
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
        lease_usage: LeaseUsage,
        lease_issuer_route: Option<MultiAddr>,
    ) -> miette::Result<Reply<InletStatus>>;

    #[allow(clippy::too_many_arguments)]
    async fn create_influxdb_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        influxdb_config: InfluxDBOutletConfig,
    ) -> miette::Result<OutletStatus>;
}

#[async_trait]
impl InfluxDBPortals for BackgroundNodeClient {
    #[instrument(skip(self, ctx))]
    #[allow(clippy::too_many_arguments)]
    async fn create_influxdb_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        influxdb_config: InfluxDBOutletConfig,
    ) -> miette::Result<OutletStatus> {
        let mut outlet_payload = CreateOutlet::new(to, tls, from.cloned(), true, false);
        if let Some(policy_expression) = policy_expression {
            outlet_payload.set_policy_expression(policy_expression);
        }
        let payload = CreateInfluxDBOutlet::new(outlet_payload, influxdb_config);
        let req = Request::post("/node/influxdb_outlet").body(payload);
        self.ask(ctx, req).await
    }

    #[instrument(skip(self, ctx))]
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
        lease_usage: LeaseUsage,
        lease_issuer_route: Option<MultiAddr>,
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
            );
            let payload = CreateInfluxDBInlet::new(inlet_payload, lease_usage, lease_issuer_route);
            Request::post("/node/influxdb_inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }
}

/// Request body to create an influxdb inlet
#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInfluxDBInlet {
    #[n(1)] pub(crate) tcp_inlet: CreateInlet,
    #[n(2)] pub(crate) lease_usage: LeaseUsage,
    /// Route to the lease issuer.
    /// If not given it's derived from the outlet route
    #[n(3)] pub(crate) lease_issuer_address: Option<MultiAddr>,
}

impl CreateInfluxDBInlet {
    pub fn new(
        tcp_inlet: CreateInlet,
        lease_usage: LeaseUsage,
        lease_issuer_address: Option<MultiAddr>,
    ) -> Self {
        Self {
            tcp_inlet,
            lease_usage,
            lease_issuer_address,
        }
    }
}

/// Request body to create an influxdb outlet
#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInfluxDBOutlet {
    #[n(1)] pub(crate) tcp_outlet: CreateOutlet,
    #[n(2)] pub(crate) influxdb_config: InfluxDBOutletConfig,
}

#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct LeaseManagerConfig {
    #[n(1)] pub(crate) influxdb_org_id: String,
    #[n(2)] pub(crate) influxdb_token: String,
    #[n(3)] pub(crate) lease_permissions: String,
    #[n(4)] pub(crate) expires_in: Duration,
}

#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub enum InfluxDBOutletConfig {
    #[n(0)] StartLeaseManager(#[n(0)] LeaseManagerConfig),
    #[n(1)] OutletWithFixedToken(#[n(0)] String)
}

impl LeaseManagerConfig {
    pub fn new(
        influxdb_org_id: String,
        influxdb_token: String,
        lease_permissions: String,
        expires_in: Duration,
    ) -> Self {
        Self {
            influxdb_org_id,
            influxdb_token,
            lease_permissions,
            expires_in,
        }
    }
}

impl CreateInfluxDBOutlet {
    pub fn new(tcp_outlet: CreateOutlet, influxdb_config: InfluxDBOutletConfig) -> Self {
        Self {
            tcp_outlet,
            influxdb_config,
        }
    }
}
