use crate::identities::IdentitiesRepository;
use crate::secure_channel::local_info::IdentitySecureChannelLocalInfo;
use core::fmt::{Debug, Formatter};
use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::{boxed::Box, string::String, sync::Arc, vec::Vec};
use ockam_core::Result;
use ockam_core::{async_trait, RelayMessage};

/// Access control checking that message senders have a specific set of attributes
#[derive(Clone)]
pub struct CredentialAccessControl {
    required_attributes: Vec<(String, Vec<u8>)>,
    storage: Arc<dyn IdentitiesRepository>,
}

impl CredentialAccessControl {
    /// Create a new credential access control
    pub fn new(
        required_attributes: &[(String, Vec<u8>)],
        storage: Arc<dyn IdentitiesRepository>,
    ) -> Self {
        Self {
            required_attributes: required_attributes.to_vec(),
            storage,
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
            IdentitySecureChannelLocalInfo::find_info(relay_message.local_message())
        {
            let attributes = match self
                .storage
                .get_attributes(&msg_identity_id.their_identity_id())
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
