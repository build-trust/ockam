use crate::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_core::access_control::AccessControl;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{LocalMessage, Result};

pub struct IdentityAccessControlBuilder;

impl IdentityAccessControlBuilder {
    pub fn new_with_id(their_identity_id: IdentityIdentifier) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(vec![their_identity_id])
    }

    pub fn new_with_ids(
        identity_ids: impl Into<Vec<IdentityIdentifier>>,
    ) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(identity_ids.into())
    }

    pub fn new_with_any_id() -> IdentityAnyIdAccessControl {
        IdentityAnyIdAccessControl
    }
}

pub struct IdentityAnyIdAccessControl;

#[async_trait]
impl AccessControl for IdentityAnyIdAccessControl {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        Ok(IdentitySecureChannelLocalInfo::find_info(local_msg).is_ok())
    }
}

#[derive(Clone)]
pub struct IdentityIdAccessControl {
    identity_ids: Vec<IdentityIdentifier>,
}

impl IdentityIdAccessControl {
    pub fn new(identity_ids: Vec<IdentityIdentifier>) -> Self {
        Self { identity_ids }
    }
    fn contains(&self, their_id: &IdentityIdentifier) -> bool {
        let mut found = subtle::Choice::from(0);
        for trusted_id in &self.identity_ids {
            found |= trusted_id.ct_eq(their_id);
        }
        found.into()
    }
}

#[async_trait]
impl AccessControl for IdentityIdAccessControl {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        if let Ok(msg_identity_id) = IdentitySecureChannelLocalInfo::find_info(local_msg) {
            Ok(self.contains(msg_identity_id.their_identity_id()))
        } else {
            Ok(false)
        }
    }
}
