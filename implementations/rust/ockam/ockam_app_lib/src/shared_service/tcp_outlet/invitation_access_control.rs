use crate::invitations::state::InvitationState;
use crate::state::AppState;
use ockam::abac::Abac;
use ockam::identity::{Identifier, IdentitiesAttributes};
use ockam::Context;
use ockam_core::errcode::Origin;
use ockam_core::{
    async_trait, Address, DenyAll, IncomingAccessControl, OutgoingAccessControl, RelayMessage,
    SecureChannelLocalInfo,
};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Clone)]
pub(super) struct InvitationAccessControl {
    outlet_worker_addr: Address,
    identities_attributes: Arc<IdentitiesAttributes>,
    invitations: Arc<RwLock<InvitationState>>,
    local_identity: Identifier,
    authority_identifier: Identifier,
}

impl Debug for InvitationAccessControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvitationAccessControl")
            .field("outlet_worker_addr", &self.outlet_worker_addr)
            .field("local_identity", &self.local_identity)
            .field("authority_identifier", &self.authority_identifier)
            .finish()
    }
}

impl InvitationAccessControl {
    fn new(
        outlet_worker_addr: Address,
        identities_attributes: Arc<IdentitiesAttributes>,
        invitations: Arc<RwLock<InvitationState>>,
        local_identity: Identifier,
        authority_identifier: Identifier,
    ) -> Self {
        Self {
            outlet_worker_addr,
            identities_attributes,
            invitations,
            local_identity,
            authority_identifier,
        }
    }
}

impl AppState {
    pub(super) async fn create_invitations_access_control(
        &self,
        outlet_worker_addr: Address,
    ) -> ockam_core::Result<InvitationAccessControl> {
        let node_manager = self.node_manager().await;
        let identities_attributes = node_manager
            .secure_channels()
            .identities()
            .identities_attributes();
        let invitations = self.invitations();
        let local_identity = self
            .state()
            .await
            .get_or_create_default_named_identity()
            .await?
            .identifier();

        let project_authority =
            node_manager
                .project_authority()
                .ok_or(ockam_core::Error::new_unknown(
                    Origin::Application,
                    "NodeManager has no authority",
                ))?;

        Ok(InvitationAccessControl::new(
            outlet_worker_addr,
            identities_attributes,
            invitations,
            local_identity,
            project_authority,
        ))
    }
}

impl InvitationAccessControl {
    pub fn create_incoming(&self) -> InvitationIncomingAccessControl {
        InvitationIncomingAccessControl {
            invitation_access_control: self.clone(),
        }
    }

    pub async fn create_outgoing(
        &self,
        ctx: &Context,
    ) -> ockam_core::Result<InvitationOutgoingAccessControl> {
        let ctx = ctx
            .new_detached(
                Address::random_tagged("InvitationOutgoingAccessControl"),
                DenyAll,
                DenyAll,
            )
            .await?;

        Ok(InvitationOutgoingAccessControl {
            ctx,
            invitation_access_control: self.clone(),
        })
    }

    pub async fn is_authorized(&self, identifier: &Identifier) -> ockam_core::Result<bool> {
        // allows messages when they come from our own local identity
        if identifier == &self.local_identity {
            return Ok(true);
        }

        let attributes = match self
            .identities_attributes
            .get_attributes(identifier, &self.authority_identifier)
            .await?
        {
            Some(a) => a,
            None => {
                warn!("No attributes found for identity {}", identifier);
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
                access_details.service_name().as_deref().unwrap_or("")
                    == self.outlet_worker_addr.address()
            })
        {
            return Ok(true);
        }

        warn!("No matching invitation found");
        Ok(false)
    }
}

#[derive(Debug)]
pub struct InvitationIncomingAccessControl {
    invitation_access_control: InvitationAccessControl,
}

#[async_trait]
impl IncomingAccessControl for InvitationIncomingAccessControl {
    async fn is_authorized(&self, relay_message: &RelayMessage) -> ockam_core::Result<bool> {
        if let Ok(msg_identity_id) =
            SecureChannelLocalInfo::find_info(relay_message.local_message())
        {
            self.invitation_access_control
                .is_authorized(&msg_identity_id.their_identifier().into())
                .await
        } else {
            Ok(false)
        }
    }
}

#[derive(Debug)]
pub struct InvitationOutgoingAccessControl {
    ctx: Context,
    invitation_access_control: InvitationAccessControl,
}

#[async_trait]
impl OutgoingAccessControl for InvitationOutgoingAccessControl {
    async fn is_authorized(&self, relay_message: &RelayMessage) -> ockam_core::Result<bool> {
        let identifier = match Abac::get_outgoing_identifier(&self.ctx, relay_message).await? {
            Some(identifier) => identifier,
            None => {
                debug!("identity identifier not found; access denied");

                return Ok(false);
            }
        };

        self.invitation_access_control
            .is_authorized(&identifier)
            .await
    }
}
