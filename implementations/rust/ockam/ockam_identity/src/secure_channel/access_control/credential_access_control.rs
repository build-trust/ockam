use core::fmt::{Debug, Formatter};
use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, RelayMessage};
use ockam_core::{Result, SecureChannelLocalInfo};

use crate::{Identifier, IdentitiesAttributes};

/// Access control checking that message senders have a specific set of attributes
#[derive(Clone)]
pub struct CredentialAccessControl {
    // FIXME: Can we use ABAC instead?
    required_attributes: Vec<(Vec<u8>, Vec<u8>)>,
    authority: Identifier,
    identities_attributes: Arc<IdentitiesAttributes>,
}

impl CredentialAccessControl {
    /// Create a new credential access control
    pub fn new(
        required_attributes: &[(Vec<u8>, Vec<u8>)],
        authority: Identifier,
        identities_attributes: Arc<IdentitiesAttributes>,
    ) -> Self {
        Self {
            required_attributes: required_attributes.to_vec(),
            authority,
            identities_attributes,
        }
    }
}

impl Debug for CredentialAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let attributes = format!("{:?}", self.required_attributes.iter().map(|x| &x.0));

        f.debug_struct("Credential Access Control")
            .field("Required attributes", &attributes)
            .finish()
    }
}

#[async_trait]
impl IncomingAccessControl for CredentialAccessControl {
    async fn is_authorized(&self, relay_message: &RelayMessage) -> Result<bool> {
        if let Ok(msg_identity_id) =
            SecureChannelLocalInfo::find_info(relay_message.local_message())
        {
            let attributes = match self
                .identities_attributes
                .get_attributes(&msg_identity_id.their_identifier().into(), &self.authority)
                .await?
            {
                Some(a) => a,
                None => return Ok(false), // No attributes for that Identity
            };

            for required_attribute in self.required_attributes.iter() {
                let attr_val = match attributes.attrs().get(&required_attribute.0) {
                    Some(v) => v,
                    None => return Ok(false), // No required key
                };

                if &required_attribute.1 != attr_val {
                    return Ok(false); // Value doesn't match
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
