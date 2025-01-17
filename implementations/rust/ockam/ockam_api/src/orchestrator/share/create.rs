use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::Result;

use super::{RoleInShare, ShareScope};
use crate::cli_state::{CliState, EnrollmentTicket};
use crate::error::ApiError;
use crate::orchestrator::email_address::EmailAddress;
use ockam::identity::Identifier;

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct CreateInvitation {
    #[n(1)] pub expires_at: Option<String>,
    #[n(2)] pub grant_role: RoleInShare,
    #[n(3)] pub recipient_email: EmailAddress,
    #[n(4)] pub remaining_uses: Option<usize>,
    #[n(5)] pub scope: ShareScope,
    #[n(6)] pub target_id: String,
}

#[derive(Clone, Debug, Encode, Decode, CborLen, Deserialize, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct CreateServiceInvitation {
    #[n(1)] pub expires_at: Option<String>,
    #[n(2)] pub project_id: String,
    #[n(3)] pub recipient_email: EmailAddress,

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
        recipient_email: EmailAddress,
        node_name: S,
        service_route: S,
        enrollment_ticket: EnrollmentTicket,
    ) -> Result<Self> {
        let node_identifier = cli_state.get_node(node_name.as_ref()).await?.identifier();
        let project = cli_state
            .projects()
            .get_project_by_name(project_name.as_ref())
            .await?;
        let project_authority_route = project.authority_multiaddr()?;
        Ok(CreateServiceInvitation {
            enrollment_ticket: enrollment_ticket.export()?.to_string(),
            expires_at,
            project_id: project.project_id().to_string(),
            recipient_email: recipient_email.clone(),
            project_identity: project
                .project_identifier()
                .ok_or_else(|| ApiError::core("no project identifier"))?,
            project_route: project.project_multiaddr()?.to_string(),
            project_authority_identity: project
                .authority_identifier()
                .ok_or_else(|| ApiError::core("no authority identifier"))?,
            project_authority_route: project_authority_route.to_string(),
            shared_node_identity: node_identifier,
            shared_node_route: service_route.as_ref().to_string(),
        })
    }
}
