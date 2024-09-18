use ockam::{route, Result};
use ockam_core::api::{Error, Response};
use ockam_node::Context;

use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NodeManagerWorker;

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
            tls_certificate_provider,
        } = create_inlet;
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                route![],
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
                tls_certificate_provider,
            )
            .await
        {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
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
