use crate::invitations::state::InvitationState;
use crate::state::AppState;
use ockam::identity::{Identifier, IdentityAttributesRepository, IdentitySecureChannelLocalInfo};
use ockam_core::{async_trait, IncomingAccessControl, RelayMessage};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Clone)]
pub(super) struct InvitationAccessControl {
    portal_name: String,
    identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
    invitations: Arc<RwLock<InvitationState>>,
    local_identity: Identifier,
}

impl Debug for InvitationAccessControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvitationAccessControl")
            .field("portal_name", &self.portal_name)
            .field("local_identity", &self.local_identity)
            .finish()
    }
}

impl InvitationAccessControl {
    fn new(
        portal_name: String,
        identity_attributes_repository: Arc<dyn IdentityAttributesRepository>,
        invitations: Arc<RwLock<InvitationState>>,
        local_identity: Identifier,
    ) -> Self {
        Self {
            portal_name,
            identity_attributes_repository,
            invitations,
            local_identity,
        }
    }
}

impl AppState {
    pub(super) async fn create_invitations_access_control(
        &self,
        portal_name: String,
    ) -> ockam_core::Result<Arc<InvitationAccessControl>> {
        let identity_attributes_repository = self
            .node_manager()
            .await
            .secure_channels()
            .identities()
            .identity_attributes_repository();
        let invitations = self.invitations();
        let local_identity = self
            .state()
            .await
            .get_or_create_default_named_identity()
            .await?
            .identifier();
        Ok(Arc::new(InvitationAccessControl::new(
            portal_name,
            identity_attributes_repository,
            invitations,
            local_identity,
        )))
    }
}

#[async_trait]
impl IncomingAccessControl for InvitationAccessControl {
    async fn is_authorized(&self, relay_message: &RelayMessage) -> ockam_core::Result<bool> {
        if let Ok(msg_identity_id) =
            IdentitySecureChannelLocalInfo::find_info(relay_message.local_message())
        {
            // allows messages when they comes from our own local identity
            if msg_identity_id.their_identity_id() == self.local_identity {
                return Ok(true);
            }

            let attributes = match self
                .identity_attributes_repository
                .get_attributes(&msg_identity_id.their_identity_id())
                .await?
            {
                Some(a) => a,
                None => {
                    warn!(
                        "No attributes found for identity {}",
                        msg_identity_id.their_identity_id()
                    );
                    return Ok(false);
                }
            };

            let email = if let Some(email) = attributes.attrs().get("invitation_email".as_bytes()) {
                if let Ok(email) = std::str::from_utf8(email) {
                    email.to_string()
                } else {
                    warn!("Invalid UTF8 in invitation email attribute");
                    return Ok(false);
                }
            } else {
                warn!("No invitation email attribute found");
                return Ok(false);
            };

            let invitations = self.invitations.read().await;
            if invitations
                .sent
                .iter()
                .filter(|invitation| invitation.recipient_email.to_string() == email)
                .filter_map(|sent_invitations| sent_invitations.access_details.as_ref())
                .filter(|access_details| access_details.shared_node_identity == self.local_identity)
                .any(|access_details| {
                    access_details.service_name().as_deref().unwrap_or("") == self.portal_name
                })
            {
                return Ok(true);
            }

            warn!("No matching invitation found");
            Ok(false)
        } else {
            Ok(false)
        }
    }
}
