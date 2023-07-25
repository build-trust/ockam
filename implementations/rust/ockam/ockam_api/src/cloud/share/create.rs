use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam_identity::IdentityIdentifier;

use super::{RoleInShare, SentInvitation, ShareScope};

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct CreateInvitation {
    #[n(1)] pub expires_at: Option<String>,
    #[n(2)] pub grant_role: RoleInShare,
    #[n(3)] pub recipient_email: Option<String>,
    #[n(4)] pub remaining_uses: Option<usize>,
    #[n(5)] pub scope: ShareScope,
    #[n(6)] pub target_id: String,
}

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct CreateServiceInvitation {
    #[n(1)] pub expires_at: Option<String>,
    #[n(2)] pub project_id: String,
    #[n(3)] pub recipient_email: String,

    // TODO: Should route be a MultiAddr?
    #[n(4)] pub project_identity: IdentityIdentifier,
    #[n(5)] pub project_route: String,
    #[n(6)] pub project_authority_identity: IdentityIdentifier,
    #[n(7)] pub project_authority_route: String,
    #[n(8)] pub shared_node_identity: IdentityIdentifier,
    #[n(9)] pub shared_node_route: String,
}

mod node {
    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::Context;
    use tracing::trace;

    use crate::cloud::CloudRequestWrapper;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const API_SERVICE: &str = "users";

    impl NodeManager {
        pub async fn create_invitation(
            &self,
            ctx: &Context,
            req: CreateInvitation,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<SentInvitation> {
            Response::parse_response_body(
                self.create_invitation_response(
                    ctx,
                    CloudRequestWrapper::new(req, route, identity_name),
                )
                .await?
                .as_slice(),
            )
        }

        pub(crate) async fn create_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateInvitation>,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "create_invitation";
            trace!(%req_body.scope, target_id = %req_body.target_id, "creating invitation");

            let req_builder = Request::post("/v0/invites").body(req_body);

            self.request_controller(
                ctx,
                label,
                "create_invitation",
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }

        pub async fn create_service_invitation(
            &self,
            ctx: &Context,
            req: CreateServiceInvitation,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<SentInvitation> {
            Response::parse_response_body(
                self.create_service_invitation_response(
                    ctx,
                    CloudRequestWrapper::new(req, route, identity_name),
                )
                .await?
                .as_slice(),
            )
        }

        pub(crate) async fn create_service_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateServiceInvitation>,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "create_service_invitation";
            trace!(project_id = %req_body.project_id, "creating service invitation");

            let req_builder = Request::post("/v0/invites/service").body(req_body);

            self.request_controller(
                ctx,
                label,
                "create_service_invitation",
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
    }

    impl NodeManagerWorker {
        pub async fn create_invitation(
            &self,
            ctx: &Context,
            req: CreateInvitation,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<SentInvitation> {
            let node_manager = self.inner().read().await;
            node_manager
                .create_invitation(ctx, req, route, identity_name)
                .await
        }

        pub(crate) async fn create_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateInvitation>,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .create_invitation_response(ctx, req_wrapper)
                .await
        }

        pub async fn create_service_invitation(
            &self,
            ctx: &Context,
            req: CreateServiceInvitation,
            route: &MultiAddr,
            identity_name: Option<String>,
        ) -> Result<SentInvitation> {
            let node_manager = self.inner().read().await;
            node_manager
                .create_service_invitation(ctx, req, route, identity_name)
                .await
        }

        pub(crate) async fn create_service_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateServiceInvitation>,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .create_service_invitation_response(ctx, req_wrapper)
                .await
        }
    }
}
