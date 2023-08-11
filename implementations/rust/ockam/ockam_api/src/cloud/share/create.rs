use minicbor::{Decode, Encode};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::error::ApiError;
use ockam_identity::{identities, IdentityIdentifier};

use super::{RoleInShare, SentInvitation, ShareScope};

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct CreateInvitation {
    #[n(1)] pub expires_at: Option<String>,
    #[n(2)] pub grant_role: RoleInShare,
    #[n(3)] pub recipient_email: String,
    #[n(4)] pub remaining_uses: Option<usize>,
    #[n(5)] pub scope: ShareScope,
    #[n(6)] pub target_id: String,
}

#[derive(Clone, Debug, Decode, Encode, Deserialize, Serialize)]
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

impl CreateServiceInvitation {
    pub async fn new<S: AsRef<str>>(
        cli_state: &CliState,
        expires_at: Option<String>,
        project_name: S,
        recipient_email: S,
        node_name: S,
        service_route: S,
    ) -> Result<Self> {
        let node_identifier = cli_state.nodes.get(node_name)?.config().identifier()?;
        let project = cli_state.projects.get(&project_name)?.config().clone();
        let project_authority_route = project
            .authority_access_route
            .ok_or(ApiError::generic("Project authority route is missing"))?;
        let project_authority_identifier = {
            let identity = project
                .authority_identity
                .ok_or(ApiError::generic("Project authority identifier is missing"))?;
            let as_hex = hex::decode(identity.as_str()).map_err(|_| {
                ApiError::generic("Project authority identifier is not a valid hex string")
            })?;
            identities()
                .identities_creation()
                .decode_identity(&as_hex)
                .await?
                .identifier()
        };
        Ok(CreateServiceInvitation {
            expires_at,
            project_id: project.id.to_string(),
            recipient_email: recipient_email.as_ref().to_string(),
            project_identity: project
                .identity
                .ok_or(ApiError::generic("Project identity is missing"))?,
            project_route: project.access_route,
            project_authority_identity: project_authority_identifier,
            project_authority_route,
            shared_node_identity: node_identifier,
            shared_node_route: service_route.as_ref().to_string(),
        })
    }
}

mod node {
    use ockam_core::api::{Request, Response};
    use ockam_core::{self};
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
