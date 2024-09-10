use crate::influxdb::token_lease_manager::InfluxDbTokenLeaseManagerWorker;
use crate::nodes::models::services::{DeleteServiceRequest, StartServiceRequest};
use crate::nodes::{InMemoryNode, NodeManagerWorker};
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Response};
use ockam_core::Address;
use ockam_node::{Context, WorkerBuilder};
use std::time::Duration;

impl NodeManagerWorker {
    pub(crate) async fn start_influxdb_token_lease_manager_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartInfluxDbLeaseManagerRequest>,
    ) -> Result<Response, Response<Error>> {
        let request = body.request().clone();
        match self
            .node_manager
            .start_influxdb_token_lease_manager_service(
                context,
                Address::from_string(body.address()),
                request,
            )
            .await
        {
            Ok(_) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(crate) async fn delete_influxdb_token_lease_manager_service(
        &self,
        context: &Context,
        req: DeleteServiceRequest,
    ) -> Result<Response, Response<Error>> {
        let address = req.address();
        match self
            .node_manager
            .delete_influxdb_token_lease_manager_service(context, address.clone())
            .await
        {
            Ok(Some(_)) => Ok(Response::ok()),
            Ok(None) => Err(Response::not_found_no_request(&format!(
                "Influxdb token lease manager service not found at address '{address}'"
            ))),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl InMemoryNode {
    async fn start_influxdb_token_lease_manager_service(
        &self,
        context: &Context,
        address: Address,
        req: StartInfluxDbLeaseManagerRequest,
    ) -> Result<(), Error> {
        let (incoming_ac, outgoing_ac) = self
            .node_manager
            .access_control(
                context,
                self.project_authority(),
                Resource::new(address.address(), ResourceType::Echoer), // TODO: ResourceType::InfluxDbTokenLeaseManager
                Action::HandleMessage,
                req.policy_expression,
            )
            .await?;
        WorkerBuilder::new(InfluxDbTokenLeaseManagerWorker::new(
            address.clone(),
            req.influxdb_org_id,
            req.influxdb_token,
            req.token_permissions,
            req.token_ttl,
        ))
        .with_address(address.clone())
        .with_incoming_access_control_arc(incoming_ac)
        .with_outgoing_access_control_arc(outgoing_ac)
        .start(context)
        .await?;
        self.registry.influxdb_services.insert(address, ()).await;
        todo!()
    }

    async fn delete_influxdb_token_lease_manager_service(
        &self,
        context: &Context,
        address: Address,
    ) -> Result<Option<()>, Error> {
        debug!(address = %address,"Deleting influxdb token lease manager service");
        match self.registry.influxdb_services.get(&address).await {
            None => Ok(None),
            Some(_) => {
                context.stop_worker(address.clone()).await?;
                self.registry.influxdb_services.remove(&address).await;
                Ok(Some(()))
            }
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub(crate) struct StartInfluxDbLeaseManagerRequest {
    #[n(1)] influxdb_org_id: String,
    #[n(2)] influxdb_token: String,
    #[n(3)] token_permissions: String,
    #[n(4)] token_ttl: Duration,
    #[n(5)] policy_expression: Option<PolicyExpression>,
}
