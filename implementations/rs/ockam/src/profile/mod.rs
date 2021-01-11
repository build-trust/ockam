use ockam_vault::{HashVault, SecretVault, SignerVault, VerifierVault};

pub mod error;
pub mod profile;
pub mod profile_event;
pub mod profile_event_binary_model;
pub mod profile_manager;

pub trait ProfileVault: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

impl<D> ProfileVault for D where D: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

#[cfg(test)]
mod tests {
    use crate::profile::profile::{ProfileEventAttributeKey, ProfileEventAttributes};
    use crate::profile::profile_manager::ProfileManager;
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
            .create_profile(Some(attributes.clone()), vault)
            .unwrap();

        let now = chrono::offset::Utc::now().timestamp();
        attributes.insert(
            ProfileEventAttributeKey::CREATION_DATE.to_string(),
            now.to_string(),
        );
        manager
            .rotate_profile(&mut profile, Some(attributes.clone()))
            .unwrap();

        let nonce = b"nonce";

        let signature = manager.attest_profile(&profile, nonce).unwrap();

        let now = chrono::offset::Utc::now().timestamp();
        attributes.insert(
            ProfileEventAttributeKey::CREATION_DATE.to_string(),
            now.to_string(),
        );
        manager
            .revoke_profile(profile, Some(attributes.clone()))
            .unwrap();
    }
}
