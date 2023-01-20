use crate::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{RelayMessage, Result};

/// Builder for `Identity`-related AccessControls
pub struct IdentityAccessControlBuilder;

impl IdentityAccessControlBuilder {
    /// `IncomingAccessControl` that checks if the author of the message possesses
    /// given `IdentityIdentifier`
    pub fn new_with_id(their_identity_id: IdentityIdentifier) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(vec![their_identity_id])
    }

    /// `IncomingAccessControl` that checks if the author of the message possesses
    /// an `IdentityIdentifier` from the pre-known list
    pub fn new_with_ids(
        identity_ids: impl Into<Vec<IdentityIdentifier>>,
    ) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(identity_ids.into())
    }

    /// `IncomingAccessControl` that checks if message was sent through a SecureChannel
    pub fn new_with_any_id() -> IdentityAnyIdAccessControl {
        IdentityAnyIdAccessControl
    }
}

/// `IncomingAccessControl` check that succeeds if message came through a SecureChannel
/// with any `IdentityIdentifier` (i.e. any SecureChannel)
#[derive(Debug)]
pub struct IdentityAnyIdAccessControl;

#[async_trait]
impl IncomingAccessControl for IdentityAnyIdAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        Ok(IdentitySecureChannelLocalInfo::find_info(relay_msg.local_message()).is_ok())
    }
}

/// `IncomingAccessControl` check that succeeds if message came from some `IdentityIdentifier`
/// from a pre-known list
#[derive(Clone, Debug)]
pub struct IdentityIdAccessControl {
    identity_ids: Vec<IdentityIdentifier>,
}

impl IdentityIdAccessControl {
    /// Constructor
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
impl IncomingAccessControl for IdentityIdAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if let Ok(msg_identity_id) =
            IdentitySecureChannelLocalInfo::find_info(relay_msg.local_message())
        {
            Ok(self.contains(msg_identity_id.their_identity_id()))
        } else {
            Ok(false)
        }
    }
}
