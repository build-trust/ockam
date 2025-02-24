use ockam::{route, Result};
use ockam_core::api::{Error, Response};
use ockam_node::Context;
use tracing::Level;

use crate::nodes::models::portal::{CreateInlet, InletStatusList, InletStatusView};
use crate::nodes::NodeManagerWorker;

impl NodeManagerWorker {
    pub(crate) async fn get_inlets(&self) -> Result<Response<InletStatusList>, Response<Error>> {
        let inlets = self.node_manager.list_inlets().await;
        Ok(Response::ok().body(InletStatusList(inlets)))
    }

    #[instrument(skip_all, level = Level::TRACE)]
    pub(crate) async fn create_inlet(
        &self,
        ctx: &Context,
        create_inlet: CreateInlet,
    ) -> Result<Response<InletStatusView>, Response<Error>> {
        let CreateInlet {
            listen_addr,
            target_redundancy,
            outlet_addresses,
            alias,
            authorized,
            ping_timeout,
            wait_for_outlet,
            policy_expression,
            wait_connection,
            secure_channel_identifier,
            enable_udp_puncture,
            disable_tcp_fallback,
            privileged,
            tls_certificate_provider,
            skip_handshake,
            enable_nagle,
            prefix_route,
        } = create_inlet;
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                prefix_route,
                route![],
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
        {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(crate) async fn delete_inlet(
        &self,
        context: &Context,
        alias: &str,
    ) -> Result<Response<InletStatusView>, Response<Error>> {
        match self.node_manager.delete_inlet(context, alias).await {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(crate) async fn show_inlet(
        &self,
        alias: &str,
    ) -> Result<Response<InletStatusView>, Response<Error>> {
        match self.node_manager.show_inlet(alias).await {
            Some(inlet) => Ok(Response::ok().body(inlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Inlet with alias {alias} not found"
            ))),
        }
    }
}
