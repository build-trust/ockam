use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, SecureChannelLocalInfo};
use ockam_core::{RelayMessage, Result};

use crate::models::Identifier;

/// Builder for `Identity`-related AccessControls
pub struct IdentityAccessControlBuilder;

impl IdentityAccessControlBuilder {
    /// `IncomingAccessControl` that checks if the author of the message possesses
    /// given `Identifier`
    pub fn new_with_id(their_identity_id: Identifier) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(vec![their_identity_id])
    }

    /// `IncomingAccessControl` that checks if the author of the message possesses
    /// an `Identifier` from the pre-known list
    pub fn new_with_ids(identity_ids: impl Into<Vec<Identifier>>) -> IdentityIdAccessControl {
        IdentityIdAccessControl::new(identity_ids.into())
    }

    /// `IncomingAccessControl` that checks if message was sent through a SecureChannel
    pub fn new_with_any_id() -> IdentityAnyIdAccessControl {
        IdentityAnyIdAccessControl
    }
}

/// `IncomingAccessControl` check that succeeds if message came through a SecureChannel
/// with any `Identifier` (i.e. any SecureChannel)
#[derive(Debug)]
pub struct IdentityAnyIdAccessControl;

#[async_trait]
impl IncomingAccessControl for IdentityAnyIdAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        Ok(SecureChannelLocalInfo::find_info(relay_msg.local_message()).is_ok())
    }
}

/// `IncomingAccessControl` check that succeeds if message came from some `Identifier`
/// from a pre-known list
#[derive(Clone, Debug)]
pub struct IdentityIdAccessControl {
    identity_ids: Vec<Identifier>,
}

impl IdentityIdAccessControl {
    /// Constructor
    pub fn new(identity_ids: Vec<Identifier>) -> Self {
        Self { identity_ids }
    }
}

#[async_trait]
impl IncomingAccessControl for IdentityIdAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if let Ok(msg_identity_id) = SecureChannelLocalInfo::find_info(relay_msg.local_message()) {
            Ok(self
                .identity_ids
                .contains(&msg_identity_id.their_identifier().into()))
        } else {
            Ok(false)
        }
    }
}
