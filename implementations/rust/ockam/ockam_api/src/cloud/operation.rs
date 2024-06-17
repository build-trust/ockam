use crate::cloud::{ControllerClient, HasSecureClient, ORCHESTRATOR_AWAIT_TIMEOUT};
use miette::{miette, IntoDiagnostic};
use minicbor::{CborLen, Decode, Encode};
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use tracing::trace;

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone)]
#[cbor(map)]
pub struct Operation {
    #[cbor(n(1))]
    pub id: String,

    #[cbor(n(2))]
    pub status: Status,
}

impl Operation {
    pub fn is_successful(&self) -> bool {
        self.status == Status::Succeeded
    }

    pub fn is_failed(&self) -> bool {
        self.status == Status::Failed
    }

    pub fn is_completed(&self) -> bool {
        self.is_successful() || self.is_failed()
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone)]
#[cbor(map)]
pub struct CreateOperationResponse {
    #[cbor(n(1))]
    pub operation_id: String,
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Status {
    #[n(0)] Started,
    #[n(1)] Succeeded,
    #[n(2)] Failed,
}

#[async_trait]
pub trait Operations {
    async fn get_operation(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<Option<Operation>>;

    async fn wait_until_operation_is_complete(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<()>;
}

const TARGET: &str = "ockam_api::cloud::operation";
const API_SERVICE: &str = "projects";

#[async_trait]
impl Operations for ControllerClient {
    #[instrument(skip_all, fields(operation_id = operation_id))]
    async fn get_operation(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<Option<Operation>> {
        trace!(target: TARGET, operation_id, "getting operation");
        let req = Request::get(format!("/v1/operations/{operation_id}"));
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .found()
            .into_diagnostic()
    }

    #[instrument(skip_all, fields(operation_id = operation_id))]
    async fn wait_until_operation_is_complete(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<()> {
        let retry_strategy = FixedInterval::from_millis(5000)
            .take((ORCHESTRATOR_AWAIT_TIMEOUT.as_millis() / 5000) as usize);
        let operation = Retry::spawn(retry_strategy.clone(), || async {
            if let Some(operation) = self.get_operation(ctx, operation_id).await? {
                if operation.is_completed() {
                    Ok(operation)
                } else {
                    Err(miette!(
                        "The operation {operation_id} did not complete in time. Please try again"
                    ))
                }
            } else {
                Err(miette!(
                    "The operation {operation_id} could not be retrieved. Please try again."
                ))
            }
        })
        .await?;

        if operation.is_successful() {
            Ok(())
        } else {
            Err(miette!(
                "The operation {operation_id} completed but was not successful."
            ))
        }
    }
}
