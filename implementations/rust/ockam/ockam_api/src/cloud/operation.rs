use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
#[cbor(map)]
pub struct Operation {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<2432199>,

    #[cbor(b(1))]
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
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<9056534>,

    #[cbor(b(1))]
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
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::BareCloudRequestWrapper;
    use crate::nodes::NodeManagerWorker;

    const TARGET: &str = "ockam_api::cloud::operation";
    const API_SERVICE: &str = "projects";

    impl NodeManagerWorker {
        pub(crate) async fn get_operation(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            operation_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "get_operation";
            trace!(target: TARGET, operation_id, "getting operation");

            let req_builder = Request::get(format!("/v1/operations/{operation_id}"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
    }
}
