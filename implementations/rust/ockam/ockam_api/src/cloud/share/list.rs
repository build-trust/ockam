use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::{InvitationWithAccess, ReceivedInvitation, SentInvitation};

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct ListInvitations {
    #[n(1)] pub kind: InvitationListKind,
}

#[derive(Clone, Debug, PartialEq, Decode, Deserialize, Encode, Serialize)]
#[cbor(index_only)]
#[rustfmt::skip]
pub enum InvitationListKind {
    #[n(0)] All,
    #[n(1)] Sent,
    #[n(2)] Received,
    #[n(3)] Accepted,
}

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct InvitationList {
    #[n(1)] pub sent: Option<Vec<SentInvitation>>,
    #[n(2)] pub received: Option<Vec<ReceivedInvitation>>,
    #[n(3)] pub accepted: Option<Vec<InvitationWithAccess>>,
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
        pub async fn list_shares(
            &self,
            ctx: &Context,
            req: ListInvitations,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<InvitationList> {
            Response::parse_response_body(
                self.list_shares_response(ctx, CloudRequestWrapper::new(req, route, identity_name))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn list_shares_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<ListInvitations>,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;
            let label = "list_shares";
            let req_builder = Request::get("/v0/invites").body(req_body);

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
        pub async fn list_shares(
            &self,
            ctx: &Context,
            req: ListInvitations,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<InvitationList> {
            let node_manager = self.inner().read().await;
            node_manager
                .list_shares(ctx, req, route, identity_name)
                .await
        }

        pub(crate) async fn list_shares_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<ListInvitations>,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.list_shares_response(ctx, req_wrapper).await
        }
    }
}
