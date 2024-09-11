use crate::cloud::lease_manager::models::influxdb::Token;
use crate::influxdb::token_lessor_worker::InfluxDbTokenLessorWorker;
use crate::nodes::models::services::{DeleteServiceRequest, StartServiceRequest};
use crate::nodes::service::messages::{Messages, SendMessage};
use crate::nodes::{InMemoryNode, NodeManagerWorker};
use crate::{ApiError, DefaultAddress};
use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::env::FromString;
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, WorkerBuilder};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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
        debug!(address = %address.address(), "Starting influxdb token lease manager service");
        let (incoming_ac, outgoing_ac) = self
            .access_control(
                context,
                self.project_authority(),
                Resource::new(address.address(), ResourceType::InfluxDbLessor),
                Action::HandleMessage,
                req.policy_expression,
            )
            .await?;

        //Taken from kafka_services.rs
        // every secure channel can reach this service
        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;
        context.flow_controls().add_consumer(
            address.clone(),
            &&default_secure_channel_listener_flow_control_id
        );

        WorkerBuilder::new(InfluxDbTokenLessorWorker::new(
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
        self.registry
            .influxdb_services
            .insert(address.clone(), ())
            .await;
        Ok(())
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

#[derive(Debug, Clone, Encode, Decode, CborLen, Serialize, Deserialize, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartInfluxDbLeaseManagerRequest {
    #[n(1)] influxdb_org_id: String,
    #[n(2)] influxdb_token: String,
    #[n(3)] token_permissions: String,
    #[n(4)] token_ttl: i32,
    #[n(5)] policy_expression: Option<PolicyExpression>,
}

#[async_trait]
pub trait InfluxDbTokenLessorNodeServiceTrait {
    async fn create_token(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<Token>;

    async fn get_token(&self, ctx: &Context, token_id: &str) -> miette::Result<Token>;

    async fn revoke_token(&self, ctx: &Context, token_id: &str) -> miette::Result<()>;

    async fn list_tokens(&self, ctx: &Context) -> miette::Result<Vec<Token>>;
}

#[async_trait]
impl InfluxDbTokenLessorNodeServiceTrait for InMemoryNode {
    async fn create_token(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<Token> {
        let req = Request::post("").to_vec().into_diagnostic()?;
        let bytes = self.send_message(ctx, at, req, None).await?;
        let res = Response::parse_response_reply::<Token>(bytes.as_slice()).into_diagnostic()?;
        Ok(res.success().into_diagnostic()?)
    }

    async fn get_token(&self, ctx: &Context, token_id: &str) -> miette::Result<Token> {
        todo!()
    }

    async fn revoke_token(&self, ctx: &Context, token_id: &str) -> miette::Result<()> {
        todo!()
    }

    async fn list_tokens(&self, ctx: &Context) -> miette::Result<Vec<Token>> {
        todo!()
    }
}
