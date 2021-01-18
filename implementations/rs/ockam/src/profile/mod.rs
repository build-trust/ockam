use ockam_vault::{HashVault, SecretVault, SignerVault, VerifierVault};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod change_event;
pub mod error;
pub mod event_handlers;
pub mod profile;
pub mod profile_manager;
pub mod signed_change_event;

pub type ProfileEventAttributes = HashMap<String, String>;
pub type ProfileEventAdditionalData = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProfileId(String);

impl AsRef<String> for ProfileId {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl ProfileId {
    pub fn from_hash(hash: &[u8]) -> Self {
        Self {
            0: format!("P_ID.{}", hex::encode(&hash)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventId(String);

impl AsRef<String> for EventId {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl EventId {
    pub fn from_hash(hash: &[u8]) -> Self {
        Self {
            0: format!("E_ID.{}", hex::encode(&hash)),
        }
    }
}

pub trait ProfileVault: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

impl<D> ProfileVault for D where D: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

#[cfg(test)]
mod tests {
    use crate::profile::change_event::{
        ProfileEventAttributeKey, ProfileKeyPurpose, ProfileKeyType,
    };
    use crate::profile::profile_manager::ProfileManager;
    use crate::profile::ProfileEventAttributes;
    use ockam_vault_software::DefaultVault;
    use std::sync::{Arc, Mutex};

    #[allow(non_snake_case)]
    #[test]
    fn test() {
        let vault = DefaultVault::default();
        let vault = Arc::new(Mutex::new(vault));
        let manager = ProfileManager::new();

        let mut attributes = ProfileEventAttributes::new();
        let now = chrono::offset::Utc::now().timestamp();
        attributes.insert(
            ProfileEventAttributeKey::CREATION_DATE.to_string(),
            now.to_string(),
        );
        attributes.insert(
            ProfileEventAttributeKey::FRIENDLY_NAME.to_string(),
            "Alice".to_string(),
        );

        let mut profile = manager
            .create_profile(
                ProfileKeyType::Main,
                ProfileKeyPurpose::Kex,
                Some(attributes.clone()),
                vault.clone(),
            )
            .unwrap();

        manager
            .create_profile_key(
                &mut profile,
                ProfileKeyType::Additional,
                ProfileKeyPurpose::Kex,
                Some(attributes.clone()),
                vault.clone(),
            )
            .unwrap();

        let now = chrono::offset::Utc::now().timestamp();
        attributes.insert(
            ProfileEventAttributeKey::CREATION_DATE.to_string(),
            now.to_string(),
        );
        manager
            .rotate_profile_key(
                &mut profile,
                ProfileKeyType::Main,
                ProfileKeyPurpose::Kex,
                Some(attributes.clone()),
                vault.clone(),
            )
            .unwrap();

        let nonce = b"nonce";

        let signature = manager
            .attest_profile(
                &profile,
                ProfileKeyType::Main,
                ProfileKeyPurpose::Kex,
                nonce,
                vault.clone(),
            )
            .unwrap();

        let now = chrono::offset::Utc::now().timestamp();
        attributes.insert(
            ProfileEventAttributeKey::CREATION_DATE.to_string(),
            now.to_string(),
        );
        manager
            .revoke_profile_key(
                &mut profile,
                ProfileKeyType::Main,
                ProfileKeyPurpose::Kex,
                Some(attributes.clone()),
                vault.clone(),
            )
            .unwrap();
    }
}
