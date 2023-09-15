use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::Result;

use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::error::ApiError;
use crate::identity::EnrollmentTicket;
use ockam::identity::{identities, Identifier};

use super::{RoleInShare, ShareScope};

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
