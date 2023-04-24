use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(map)]
pub struct Operation<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<2432199>,

    #[cbor(b(1))]
    #[serde(borrow)]
    pub id: CowStr<'a>,

    #[cbor(n(2))]
    pub status: Status,
}

impl Clone for Operation<'_> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl Operation<'_> {
    pub fn to_owned<'r>(&self) -> Operation<'r> {
        Operation {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            id: self.id.to_owned(),
            status: self.status.clone(),
        }
    }

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

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default)]
#[cbor(map)]
pub struct CreateOperationResponse<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<9056534>,

    #[cbor(b(1))]
    #[serde(borrow)]
    pub operation_id: CowStr<'a>,
}

impl Clone for CreateOperationResponse<'_> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl CreateOperationResponse<'_> {
    pub fn to_owned<'r>(&self) -> CreateOperationResponse<'r> {
        CreateOperationResponse {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            operation_id: self.operation_id.to_owned(),
        }
    }
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
