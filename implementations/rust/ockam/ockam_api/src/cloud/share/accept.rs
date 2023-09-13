use minicbor::{Decode, Encode};
use serde::Serialize;

use super::RoleInShare;

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptInvitation {
    #[n(1)] pub id: String,
}

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptedInvitation {
    #[n(1)] pub id: String,
    #[n(2)] pub scope: RoleInShare,
    #[n(3)] pub target_id: String,
}

mod node {
    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::CloudRequestWrapper;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const API_SERVICE: &str = "users";

    impl NodeManager {
        pub async fn accept_invitation(
            &self,
            ctx: &Context,
            req: AcceptInvitation,
        ) -> Result<AcceptedInvitation> {
            Response::parse_response_body(
                self.accept_invitation_response(ctx, CloudRequestWrapper::new(req))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn accept_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<AcceptInvitation>,
        ) -> Result<Vec<u8>> {
            let req = Request::post("/v0/redeem_invite").body(req_wrapper.req);
            self.make_controller_client()
                .await?
                .request(ctx, API_SERVICE, req)
                .await
        }
    }

    impl NodeManagerWorker {
        pub async fn accept_invitation(
            &self,
            ctx: &Context,
            req: AcceptInvitation,
        ) -> Result<AcceptedInvitation> {
            let node_manager = self.inner().read().await;
            node_manager.accept_invitation(ctx, req).await
        }

        pub(crate) async fn accept_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<AcceptInvitation>,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .accept_invitation_response(ctx, req_wrapper)
                .await
        }
    }
}
