use std::sync::Arc;

use ockam::{route, Address, Result};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Response};
use ockam_node::Context;
use ockam_transport_tcp::PortalInletInterceptor;

use crate::http_auth::HttpAuthInterceptorFactory;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NodeManagerWorker;
use crate::TokenLeaseRefresher;

impl NodeManagerWorker {
    pub(crate) async fn get_inlets(&self) -> Result<Response<Vec<InletStatus>>, Response<Error>> {
        let inlets = self.node_manager.list_inlets().await;
        Ok(Response::ok().body(inlets))
    }

    #[instrument(skip_all)]
    pub(crate) async fn create_inlet(
        &self,
        ctx: &Context,
        create_inlet: CreateInlet,
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
            is_http_auth_inlet,
        } = create_inlet;

        let prefix_route = if is_http_auth_inlet {
            let interceptor_address = self
                .create_http_auth_interceptor(ctx, &alias, policy_expression.clone())
                .await
                .map_err(|e| {
                    Response::bad_request_no_request(&format!(
                        "Error creating http interceptor {:}",
                        e
                    ))
                })?;
            route![interceptor_address]
        } else {
            route![]
        };
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                prefix_route,
                route![],
                outlet_addr,
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
        {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    async fn create_http_auth_interceptor(
        &self,
        ctx: &Context,
        inlet_alias: &String,
        inlet_policy_expression: Option<PolicyExpression>,
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

        let token_refresher = TokenLeaseRefresher::new(ctx, self.node_manager.clone()).await?;
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

    pub(crate) async fn delete_inlet(
        &self,
        alias: &str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        match self.node_manager.delete_inlet(alias).await {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(crate) async fn show_inlet(
        &self,
        alias: &str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        match self.node_manager.show_inlet(alias).await {
            Some(inlet) => Ok(Response::ok().body(inlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Inlet with alias {alias} not found"
            ))),
        }
    }
}
