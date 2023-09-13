use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

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

mod node {
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::nodes::{NodeManager, NodeManagerWorker};

    const TARGET: &str = "ockam_api::cloud::operation";
    const API_SERVICE: &str = "projects";

    impl NodeManagerWorker {
        pub(crate) async fn get_operation(
            &self,
            ctx: &Context,
            operation_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.get_operation(ctx, operation_id).await
        }
    }

    impl NodeManager {
        pub(crate) async fn get_operation(
            &self,
            ctx: &Context,
            operation_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, operation_id, "getting operation");
            let req = Request::get(format!("/v1/operations/{operation_id}"));
            let client = self.make_controller_client().await?;
            client.request_controller(ctx, API_SERVICE, req).await
        }
    }
}
