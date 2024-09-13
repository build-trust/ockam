use crate::influxdb::influxdb_api_client::InfluxDBApiClient;
use crate::influxdb::lease_token::LeaseToken;
use crate::influxdb::token_lessor_processor::InfluxDBTokenLessorProcessor;
use crate::influxdb::token_lessor_worker::InfluxDBTokenLessorWorker;
use crate::nodes::models::services::{DeleteServiceRequest, StartServiceRequest};
use crate::nodes::service::messages::Messages;
use crate::nodes::{InMemoryNode, NodeManagerWorker};
use crate::{ApiError, DefaultAddress};
use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Request, Response};
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use time::Duration;

impl NodeManagerWorker {
    pub(crate) async fn start_influxdb_token_lessor_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartInfluxDBLeaseManagerRequest>,
    ) -> Result<Response, Response<Error>> {
        let request = body.request().clone();
        match self
            .node_manager
            .start_influxdb_token_lessor_service(
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

    pub(crate) async fn delete_influxdb_token_lessor_service(
        &self,
        context: &Context,
        req: DeleteServiceRequest,
    ) -> Result<Response, Response<Error>> {
        let address = req.address();
        match self
            .node_manager
            .delete_influxdb_token_lessor_service(context, address.clone())
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
    async fn start_influxdb_token_lessor_service(
        &self,
        context: &Context,
        address: Address,
        req: StartInfluxDBLeaseManagerRequest,
    ) -> Result<(), Error> {
        debug!(address = %address.address(), "Starting influxdb token lease manager service");

        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;
        context.flow_controls().add_consumer(
            address.clone(),
            &default_secure_channel_listener_flow_control_id,
        );

        let (incoming_ac, outgoing_ac) = self
            .access_control(
                context,
                self.project_authority(),
                Resource::new(address.address(), ResourceType::InfluxDBLessor),
                Action::HandleMessage,
                req.policy_expression,
            )
            .await?;

        let worker = InfluxDBTokenLessorWorker::new(
            address.clone(),
            req.influxdb_address,
            req.influxdb_org_id,
            req.influxdb_token,
            req.token_permissions,
            req.token_ttl,
        )
        .await?;
        let processor = InfluxDBTokenLessorProcessor::new(worker.state.clone());

        WorkerBuilder::new(worker)
            .with_address(address.clone())
            .with_incoming_access_control_arc(incoming_ac)
            .with_outgoing_access_control_arc(outgoing_ac)
            .start(context)
            .await?;
        self.registry
            .influxdb_services
            .insert(address.clone(), ())
            .await;

        ProcessorBuilder::new(processor)
            .with_address(format!("{address}-processor"))
            .start(context)
            .await?;

        Ok(())
    }

    async fn delete_influxdb_token_lessor_service(
        &self,
        context: &Context,
        address: Address,
    ) -> Result<Option<()>, Error> {
        debug!(address = %address,"Deleting influxdb token lease manager service");
        match self.registry.influxdb_services.get(&address).await {
            None => Ok(None),
            Some(_) => {
                context.stop_worker(address.clone()).await?;
                context
                    .stop_processor(format!("{address}-processor"))
                    .await?;
                self.registry.influxdb_services.remove(&address).await;
                Ok(Some(()))
            }
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen, Serialize, Deserialize, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartInfluxDBLeaseManagerRequest {
    #[n(1)] influxdb_address: String,
    #[n(2)] influxdb_org_id: String,
    #[n(3)] influxdb_token: String,
    #[n(4)] token_permissions: String,
    #[n(5)] token_ttl: i32,
    #[n(6)] policy_expression: Option<PolicyExpression>,
}

#[async_trait]
pub trait InfluxDBTokenLessorNodeServiceTrait {
    async fn create_token(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<LeaseToken>;

    async fn get_token(
        &self,
        ctx: &Context,
        at: &MultiAddr,
        token_id: &str,
    ) -> miette::Result<LeaseToken>;

    async fn revoke_token(
        &self,
        ctx: &Context,
        at: &MultiAddr,
        token_id: &str,
    ) -> miette::Result<()>;

    async fn list_tokens(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<Vec<LeaseToken>>;
}

#[async_trait]
impl InfluxDBTokenLessorNodeServiceTrait for InMemoryNode {
    async fn create_token(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<LeaseToken> {
        let req = Request::post("").to_vec().into_diagnostic()?;
        let bytes = self.send_message(ctx, at, req, None).await?;
        let res =
            Response::parse_response_reply::<LeaseToken>(bytes.as_slice()).into_diagnostic()?;
        Ok(res.success().into_diagnostic()?)
    }

    async fn get_token(
        &self,
        ctx: &Context,
        at: &MultiAddr,
        token_id: &str,
    ) -> miette::Result<LeaseToken> {
        let req = Request::get(format!("/{token_id}"))
            .to_vec()
            .into_diagnostic()?;
        let bytes = self.send_message(ctx, at, req, None).await?;
        let res =
            Response::parse_response_reply::<LeaseToken>(bytes.as_slice()).into_diagnostic()?;
        Ok(res.success().into_diagnostic()?)
    }

    async fn revoke_token(
        &self,
        ctx: &Context,
        at: &MultiAddr,
        token_id: &str,
    ) -> miette::Result<()> {
        let req = Request::delete(format!("/{token_id}"))
            .to_vec()
            .into_diagnostic()?;
        let bytes = self.send_message(ctx, at, req, None).await?;
        let res = Response::parse_response_reply::<()>(bytes.as_slice()).into_diagnostic()?;
        Ok(res.success().into_diagnostic()?)
    }

    async fn list_tokens(&self, ctx: &Context, at: &MultiAddr) -> miette::Result<Vec<LeaseToken>> {
        let req = Request::get("").to_vec().into_diagnostic()?;
        let bytes = self.send_message(ctx, at, req, None).await?;
        let res = Response::parse_response_reply::<Vec<LeaseToken>>(bytes.as_slice())
            .into_diagnostic()?;
        Ok(res.success().into_diagnostic()?)
    }
}

pub(crate) struct InfluxDBTokenLessorState {
    #[allow(dead_code)]
    pub(super) address: Address,
    pub(super) influxdb_api_client: InfluxDBApiClient,
    pub(super) influxdb_org_id: String,

    /// Permissions for the created tokens
    pub(super) token_permissions: String,

    /// Duration for which a token is valid
    pub(super) token_ttl: Duration,

    /// Active tokens ordered by expiration time, earliest first
    pub(super) active_tokens: BinaryHeap<Reverse<LeaseToken>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_active_tokens_ordering() {
        let mut state = InfluxDBTokenLessorState {
            address: Address::from_string("test"),
            influxdb_api_client: InfluxDBApiClient::new("http://localhost:8086", "token").unwrap(),
            influxdb_org_id: "org_id".to_string(),
            token_permissions: "permissions".to_string(),
            token_ttl: Duration::seconds(60),
            active_tokens: BinaryHeap::new(),
        };

        let token1 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(10)).unix_timestamp(),
            ..Default::default()
        };
        let token2 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(20)).unix_timestamp(),
            ..Default::default()
        };
        let token3 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(30)).unix_timestamp(),
            ..Default::default()
        };
        let token4 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(40)).unix_timestamp(),
            ..Default::default()
        };
        let token5 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(50)).unix_timestamp(),
            ..Default::default()
        };
        let token6 = LeaseToken {
            expires_at: (time::OffsetDateTime::now_utc() + Duration::seconds(60)).unix_timestamp(),
            ..Default::default()
        };

        state.active_tokens.push(Reverse(token4.clone()));
        state.active_tokens.push(Reverse(token2.clone()));
        state.active_tokens.push(Reverse(token6.clone()));
        state.active_tokens.push(Reverse(token1.clone()));
        state.active_tokens.push(Reverse(token5.clone()));
        state.active_tokens.push(Reverse(token3.clone()));

        assert_eq!(state.active_tokens.pop().unwrap().0, token1);
        assert_eq!(state.active_tokens.pop().unwrap().0, token2);
        assert_eq!(state.active_tokens.pop().unwrap().0, token3);
        assert_eq!(state.active_tokens.pop().unwrap().0, token4);
        assert_eq!(state.active_tokens.pop().unwrap().0, token5);
        assert_eq!(state.active_tokens.pop().unwrap().0, token6);
    }
}
