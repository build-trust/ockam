use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::Result;

use crate::cli_state::{CliState, EnrollmentTicket};
use crate::error::ApiError;
use ockam::identity::Identifier;

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
        let node_identifier = cli_state.get_node(node_name.as_ref()).await?.identifier();
        let project = cli_state.get_project_by_name(project_name.as_ref()).await?;
        let project_authority_route = project.authority_access_route()?;
        let project_authority_identifier = project.authority_identifier().await?;
        // see also: ockam_command::project::ticket
        let enrollment_ticket = hex::encode(
            serde_json::to_vec(&enrollment_ticket)
                .map_err(|_| ApiError::core("Could not encode enrollment ticket"))?,
        );
        Ok(CreateServiceInvitation {
            enrollment_ticket,
            expires_at,
            project_id: project.id(),
            recipient_email: recipient_email.as_ref().to_string(),
            project_identity: project.identifier()?,
            project_route: project.access_route()?.to_string(),
            project_authority_identity: project_authority_identifier,
            project_authority_route: project_authority_route.to_string(),
            shared_node_identity: node_identifier,
            shared_node_route: service_route.as_ref().to_string(),
        })
    }
}
