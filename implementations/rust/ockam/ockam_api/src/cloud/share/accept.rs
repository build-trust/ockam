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
    use ockam_multiaddr::MultiAddr;
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
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<AcceptedInvitation> {
            Response::parse_response_body(
                self.accept_invitation_response(
                    ctx,
                    CloudRequestWrapper::new(req, route, identity_name),
                )
                .await?
                .as_slice(),
            )
        }

        pub(crate) async fn accept_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<AcceptInvitation>,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;
            let label = "accept_share";
            let req_builder = Request::post("/v0/redeem_invite").body(req_body);

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

    impl NodeManagerWorker {
        pub async fn accept_invitation(
            &self,
            ctx: &Context,
            req: AcceptInvitation,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<AcceptedInvitation> {
            let node_manager = self.inner().read().await;
            node_manager
                .accept_invitation(ctx, req, route, identity_name)
                .await
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
