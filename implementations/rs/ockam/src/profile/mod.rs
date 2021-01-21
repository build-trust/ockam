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
pub struct ProfileId([u8; 32]);

impl AsRef<[u8]> for ProfileId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ProfileId {
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { 0: hash }
    }

    pub fn string_representation(&self) -> String {
        format!("P_ID.{}", hex::encode(&self.0))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventId([u8; 32]);

impl AsRef<[u8]> for EventId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl EventId {
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self { 0: hash }
    }

    pub fn string_representation(&self) -> String {
        format!("E_ID.{}", hex::encode(&self.0))
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
    use ockam_vault::VerifierVault;
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
                ProfileKeyType::Additional,
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

        let public_key =
            manager.get_profile_public_key(&profile, ProfileKeyType::Main, ProfileKeyPurpose::Kex);
        assert!(public_key.is_none());

        let public_key = manager
            .get_profile_public_key(&profile, ProfileKeyType::Additional, ProfileKeyPurpose::Kex)
            .unwrap();

        let mut v = vault.lock().unwrap();

        assert!(v.verify(&signature, &public_key, nonce).is_ok());
    }
}
