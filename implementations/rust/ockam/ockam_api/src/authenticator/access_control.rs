use crate::authenticator::MembersStorage;
use ockam::identity::IdentitySecureChannelLocalInfo;
use ockam_core::{async_trait, IncomingAccessControl, RelayMessage, Result};
use std::sync::Arc;

/// Attribute key for ockam role.
pub const OCKAM_ROLE: &[u8] = b"ockam-role";

/// Attribute value for ockam role for enrollers.
pub const ENROLLER_ROLE: &[u8] = b"enroller";

/// Incoming Access that only allows Enrollers
#[derive(Debug)]
pub struct EnrollersOnlyAccessControl {
    members_storage: Arc<dyn MembersStorage>,
}

impl EnrollersOnlyAccessControl {
    pub fn new(members_storage: Arc<dyn MembersStorage>) -> Self {
        Self { members_storage }
    }
}

#[async_trait]
impl IncomingAccessControl for EnrollersOnlyAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let identifier = if let Ok(info) =
            IdentitySecureChannelLocalInfo::find_info(relay_msg.local_message())
        {
            info.their_identity_id()
        } else {
            return Ok(false);
        };

        let member = if let Some(member) = self.members_storage.get_member(&identifier).await? {
            member
        } else {
            return Ok(false);
        };

        let role = if let Some(role) = member.attributes().get(&OCKAM_ROLE.to_vec()) {
            role
        } else {
            return Ok(false);
        };

        let res = role == ENROLLER_ROLE;

        Ok(res)
    }
}
