use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::Result;

use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::error::ApiError;
use crate::identity::EnrollmentTicket;
use ockam::identity::{identities, Identifier};

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
    #[n(4)] pub project_identity: Identifier,
    #[n(5)] pub project_route: String,
    #[n(6)] pub project_authority_identity: Identifier,
    #[n(7)] pub project_authority_route: String,
    #[n(8)] pub shared_node_identity: Identifier,
    #[n(9)] pub shared_node_route: String,
    #[n(10)] pub enrollment_ticket: String,
}

impl CreateServiceInvitation {
    pub async fn new<S: AsRef<str>>(
        cli_state: &CliState,
        expires_at: Option<String>,
        project_name: S,
        recipient_email: S,
        node_name: S,
        service_route: S,
        enrollment_ticket: EnrollmentTicket,
    ) -> Result<Self> {
        let node_identifier = cli_state.nodes.get(node_name)?.config().identifier()?;
        let project = cli_state.projects.get(&project_name)?.config().clone();
        let project_authority_route = project
            .authority_access_route
            .ok_or(ApiError::core("Project authority route is missing"))?;
        let project_authority_identifier = {
            let identity = project
                .authority_identity
                .ok_or(ApiError::core("Project authority identifier is missing"))?;
            let as_hex = hex::decode(identity.as_str()).map_err(|_| {
                ApiError::core("Project authority identifier is not a valid hex string")
            })?;
            identities()
                .identities_creation()
                .import(None, &as_hex)
                .await?
                .identifier()
                .clone()
        };
        // see also: ockam_command::project::ticket
        let enrollment_ticket = hex::encode(
            serde_json::to_vec(&enrollment_ticket)
                .map_err(|_| ApiError::core("Could not encode enrollment ticket"))?,
        );
        Ok(CreateServiceInvitation {
            enrollment_ticket,
            expires_at,
            project_id: project.id.to_string(),
            recipient_email: recipient_email.as_ref().to_string(),
            project_identity: project
                .identity
                .ok_or(ApiError::core("Project identity is missing"))?,
            project_route: project.access_route,
            project_authority_identity: project_authority_identifier,
            project_authority_route,
            shared_node_identity: node_identifier,
            shared_node_route: service_route.as_ref().to_string(),
        })
    }
}

mod node {
    use tracing::trace;

    use ockam_core::api::{Request, Response};
    use ockam_core::{self};
    use ockam_node::Context;

    use crate::cloud::CloudRequestWrapper;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const API_SERVICE: &str = "users";

    impl NodeManager {
        pub async fn create_invitation(
            &self,
            ctx: &Context,
            req: CreateInvitation,
        ) -> Result<SentInvitation> {
            Response::parse_response_body(
                self.create_invitation_response(ctx, CloudRequestWrapper::new(req))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn create_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateInvitation>,
        ) -> Result<Vec<u8>> {
            let req_body = req_wrapper.req;
            trace!(%req_body.scope, target_id = %req_body.target_id, "creating invitation");
            let req = Request::post("/v0/invites").body(req_body);

            self.request_controller(ctx, API_SERVICE, req).await
        }

        pub async fn create_service_invitation(
            &self,
            ctx: &Context,
            req: CreateServiceInvitation,
        ) -> Result<SentInvitation> {
            Response::parse_response_body(
                self.create_service_invitation_response(ctx, CloudRequestWrapper::new(req))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn create_service_invitation_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateServiceInvitation>,
        ) -> Result<Vec<u8>> {
            let req_body = req_wrapper.req;
            trace!(project_id = %req_body.project_id, "creating service invitation");
            let req = Request::post("/v0/invites/service").body(req_body);

            self.request_controller(ctx, API_SERVICE, req).await
        }
    }

    impl NodeManagerWorker {
        pub async fn create_invitation(
            &self,
            ctx: &Context,
            req: CreateInvitation,
        ) -> Result<SentInvitation> {
            let node_manager = self.inner().read().await;
            node_manager.create_invitation(ctx, req).await
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
        ) -> Result<SentInvitation> {
            let node_manager = self.inner().read().await;
            node_manager.create_service_invitation(ctx, req).await
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
