use crate::{ProfileIdentifier, ProfileTrait};

pub mod authentication;
pub use authentication::*;
pub mod change;
pub use change::*;
pub mod contacts;
pub use contacts::*;
pub mod identifiers;
pub use identifiers::*;
pub mod secrets;
pub use secrets::*;

/// An Entity represents an identity in various authentication contexts.
#[derive(Clone)]
pub struct Entity<P: ProfileTrait> {
    default_profile_identifier: ProfileIdentifier,
    profiles: Vec<P>,
}

impl<P: ProfileTrait> Entity<P> {
    /// Create a new Entity with the given default profile.
    pub fn new(default_profile: P) -> Self {
        let idref = default_profile.identifier().unwrap();
        let default_profile_identifier = ProfileIdentifier::from_key_id(idref.key_id().clone());
        let profiles = vec![default_profile];
        Entity {
            default_profile_identifier,
            profiles,
        }
    }

    fn default_profile(&self) -> Option<&P> {
        self.profiles
            .iter()
            .find(|profile| self.default_profile_identifier == profile.identifier().unwrap())
    }

    fn _add_profile(&mut self, profile: P) {
        self.profiles.push(profile);
    }
}

#[cfg(test)]
#[allow(unreachable_code, unused_variables)]
mod test {
    use crate::{
        Entity, KeyAttributes, Profile, ProfileAuth, ProfileContacts, ProfileIdentity,
        ProfileSecrets, ProfileSync, ProfileTrait,
    };
    use ockam_node::Context;
    use ockam_vault_sync_core::Vault;

    async fn new_entity(ctx: &Context) -> ockam_core::Result<Entity<ProfileSync>> {
        let vault = Vault::create(ctx)?;

        let profile = Profile::create(&ctx, &vault).await;
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        Ok(Entity::new(profile))
    }

    #[test]
    fn test_new_entity() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let e = new_entity(&ctx).await.unwrap();
                assert!(!e
                    .default_profile_identifier
                    .to_string_representation()
                    .is_empty());
                assert!(!e.profiles.is_empty());

                let default = e.default_profile();

                assert!(default.is_some());
                ctx.stop().await.unwrap();
            })
            .unwrap();
    }

    fn entity_auth_tests<P: ProfileTrait>(mut e: Entity<P>) -> ockam_core::Result<()> {
        let channel_state = "test".as_bytes();
        let proof = e.generate_authentication_proof(channel_state);
        assert!(proof.is_ok());

        let proof = proof.unwrap();

        // TODO WIP: Need Contacts for this test to be successful. This tests the delegation but not correct operation currently.
        let default_id = e.default_profile_identifier.clone();
        let valid = e.verify_authentication_proof(channel_state, &default_id, proof.as_slice());
        // assert!(valid.is_ok());
        Ok(())
    }

    fn entity_change_tests<P: ProfileTrait>(e: Entity<P>) -> ockam_core::Result<()> {
        // TODO WIP: Need key ops and other event generating APIs to easily test this
        // change_events update_no_verification verify
        Ok(())
    }

    async fn entity_contacts_tests<P: ProfileTrait>(
        ctx: &Context,
        mut e: Entity<P>,
    ) -> ockam_core::Result<()> {
        //    verify_and_update_contact

        let alice = new_entity(&ctx).await.unwrap();
        let alice_id = alice.identifier()?.clone();

        let alice_contact = alice.serialize_to_contact()?;
        let alice_contact = Profile::deserialize_contact(alice_contact.as_slice())?;

        let to_alice_contact = alice.to_contact()?;
        assert_eq!(alice_contact.identifier(), to_alice_contact.identifier());

        e.verify_contact(&alice_contact)?;

        e.verify_and_add_contact(alice_contact)?;

        assert_eq!(1, e.contacts()?.len());

        let get_alice_contact = e.get_contact(&alice_id)?;
        assert!(get_alice_contact.is_some());

        let get_alice_contact = get_alice_contact.unwrap();
        assert_eq!(&alice_id, get_alice_contact.identifier());

        // TODO WIP: after change event emitting APIs are done, make this a non-trivial test
        let change_events = vec![];
        e.verify_and_update_contact(&alice_id, change_events)?;
        Ok(())
    }

    fn entity_secrets_test<P: ProfileTrait>(mut e: Entity<P>) -> ockam_core::Result<()> {
        //   get_secret_key  get_root_secret rotate_key

        let key_attributes = KeyAttributes::new("label".to_string());
        e.create_key(key_attributes.clone(), None)?;

        let pubkey = e.get_public_key(&key_attributes)?;
        let secret = e.get_secret_key(&key_attributes)?;
        let root = e.get_root_secret()?;

        let root_key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());

        e.rotate_key(root_key_attributes, None)?;

        /* Uncomment once rotate_key is implemented
        let new_pubkey = e.get_public_key(&key_attributes)?;

        let new_secret = e.get_secret_key(&key_attributes)?;

        assert_ne!(new_pubkey, pubkey);
        assert_ne!(new_secret, secret);
         */
        Ok(())
    }

    async fn entity_profile_mgmt_test<P: ProfileTrait>(
        ctx: &Context,
        e: Entity<P>,
    ) -> ockam_core::Result<()> {
        let vault = Vault::create(ctx)?;
        let bank_profile = Profile::create(ctx, &vault).await?;

        //    e.add_profile(bank_profile);
        Ok(())
    }

    async fn entity_all_tests(mut ctx: Context) -> ockam_core::Result<()> {
        let e = new_entity(&ctx).await?;
        entity_contacts_tests(&ctx, e.clone()).await?;
        entity_auth_tests(e.clone())?;
        entity_change_tests(e.clone())?;
        entity_secrets_test(e.clone())?;
        entity_profile_mgmt_test(&ctx, e).await?;
        ctx.stop().await
    }

    #[test]
    fn test_entity_default_profile_delegation() {
        let (ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move { entity_all_tests(ctx).await })
            .unwrap();
    }
}
