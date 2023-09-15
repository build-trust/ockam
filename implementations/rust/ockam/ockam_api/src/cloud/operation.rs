use crate::cloud::Controller;
use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
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

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default, Clone)]
#[cbor(map)]
pub struct CreateOperationResponse {
    #[cbor(n(1))]
    pub operation_id: String,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
}

const TARGET: &str = "ockam_api::cloud::operation";
const API_SERVICE: &str = "projects";

#[async_trait]
impl Operations for Controller {
    async fn get_operation(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<Option<Operation>> {
        trace!(target: TARGET, operation_id, "getting operation");
        let req = Request::get(format!("/v1/operations/{operation_id}"));
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .found()
            .into_diagnostic()
    }
}
