use ockam_vault::{HashVault, SecretVault, SignerVault, VerifierVault};

pub mod contact;
pub mod contact_event;
pub mod contact_manager;
pub mod error;

pub trait ContactVault: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

impl<D> ContactVault for D where D: SecretVault + SignerVault + VerifierVault + HashVault + Send {}

#[cfg(test)]
mod tests {
    use crate::contact::contact::ContactTags;
    use crate::contact::contact_manager::ContactManager;
    use crate::profile::profile::{ProfileEventAttributeKey, ProfileEventAttributes};
    use crate::profile::profile_manager::ProfileManager;
    use ockam_vault_software::DefaultVault;
    use std::sync::{Arc, Mutex};

    #[allow(non_snake_case)]
    #[test]
    fn test() {
        let vault = DefaultVault::default();
        let vault = Arc::new(Mutex::new(vault));
        let alice_profile_manager = ProfileManager::new();
        let device_profile_manager = ProfileManager::new();
        let mut alice_contact_manager = ContactManager::new();
        let device_contact_manager = ContactManager::new();

        let device_profile = device_profile_manager
            .create_profile(None, vault.clone())
            .unwrap();

        let device_contact = device_contact_manager
            .create_contact_from_profile(&device_profile, vault.clone())
            .unwrap();

        let mut tags = ContactTags::new();
        tags.insert("INFO".to_string(), "This is my car".to_string());

        let device_id = alice_contact_manager
            .import_contact(device_contact, tags)
            .unwrap();

        let device_public_key = alice_contact_manager.get_contact_public_key(&device_id);

        assert!(device_public_key.is_some())
    }
}
