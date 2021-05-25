use crate::{ProfileIdentifier, ProfileTrait, Result};

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

use crate::EntityError::{InvalidInternalState, InvalidParameter};
use ockam_core::hashbrown::hash_map::HashMap;

/// An Entity represents an identity in various authentication contexts.
#[derive(Clone)]
pub struct ProfileSet<P: ProfileTrait> {
    default_profile_identifier: ProfileIdentifier,
    profiles: HashMap<ProfileIdentifier, P>,
}

pub trait ProfileRetrieve<P: ProfileTrait> {
    fn profile(&self, profile_identifier: &ProfileIdentifier) -> Option<&P>;
}

pub trait ProfileAdd<P: ProfileTrait> {
    fn add_profile(&mut self, profile: P) -> Result<()>;
}

pub trait ProfileUpdate<P: ProfileTrait> {
    fn update_profile(&mut self, old_profile_id: &ProfileIdentifier, profile: P) -> Result<()>;
}

pub trait ProfileRemove {
    fn remove_profile(&mut self, profile_id: &ProfileIdentifier) -> Result<()>;
}

pub trait ProfileManagement<P: ProfileTrait>:
    ProfileRetrieve<P> + ProfileAdd<P> + ProfileUpdate<P> + ProfileRemove
{
}

impl<P: ProfileTrait, U> ProfileManagement<P> for U where
    U: ProfileRetrieve<P> + ProfileAdd<P> + ProfileUpdate<P> + ProfileRemove
{
}

impl<P: ProfileTrait> ProfileSet<P> {
    /// Create a new Entity with the given default profile.
    pub fn new(default_profile: P) -> Self {
        let idref = default_profile.identifier().unwrap();
        let default_profile_identifier = ProfileIdentifier::from_key_id(idref.key_id().clone());

        let mut profiles = HashMap::new();
        profiles.insert(default_profile_identifier.clone(), default_profile);

        ProfileSet {
            default_profile_identifier,
            profiles,
        }
    }

    fn default_profile(&self) -> Option<&P> {
        self.profile(&self.default_profile_identifier)
    }
}

impl<P: ProfileTrait> ProfileAdd<P> for ProfileSet<P> {
    fn add_profile(&mut self, profile: P) -> Result<()> {
        if let Ok(id) = profile.identifier() {
            if let Some(_) = self.profiles.insert(id, profile) {
                return Ok(());
            }
        }
        Err(InvalidInternalState.into())
    }
}

impl<P: ProfileTrait> ProfileRetrieve<P> for ProfileSet<P> {
    fn profile(&self, profile_identifier: &ProfileIdentifier) -> Option<&P> {
        self.profiles.get(profile_identifier)
    }
}

impl<P: ProfileTrait> ProfileRemove for ProfileSet<P> {
    fn remove_profile(&mut self, profile_id: &ProfileIdentifier) -> Result<()> {
        if &self.default_profile_identifier == profile_id {
            return Err(InvalidParameter.into());
        }
        if let Some(_) = self.profiles.remove(&profile_id) {
            Ok(())
        } else {
            Err(InvalidInternalState.into())
        }
    }
}

impl<P: ProfileTrait> ProfileUpdate<P> for ProfileSet<P> {
    fn update_profile(&mut self, old_profile_id: &ProfileIdentifier, profile: P) -> Result<()> {
        self.remove_profile(&old_profile_id)?;
        self.add_profile(profile)
    }
}

#[cfg(test)]
#[allow(unreachable_code, unused_variables)]
mod test {
    use crate::{
        KeyAttributes, Profile, ProfileAuth, ProfileContacts, ProfileIdentity, ProfileSecrets,
        ProfileSet, ProfileSync, ProfileTrait,
    };
    use ockam_node::Context;
    use ockam_vault_sync_core::Vault;

    async fn new_ps(ctx: &Context) -> ockam_core::Result<ProfileSet<ProfileSync>> {
        let vault = Vault::create(ctx)?;

        let profile = Profile::create(&ctx, &vault).await;
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        Ok(ProfileSet::new(profile))
    }

    #[test]
    fn test_new_ps() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let e = new_ps(&ctx).await.unwrap();
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

    fn ps_auth_tests<P: ProfileTrait>(mut e: ProfileSet<P>) -> ockam_core::Result<()> {
        let channel_state = "test".as_bytes();
        let proof = e.generate_authentication_proof(channel_state);
        assert!(proof.is_ok());

        let proof = proof.unwrap();

        let default_id = e.default_profile_identifier.clone();
        let valid = e.verify_authentication_proof(channel_state, &default_id, proof.as_slice());
        // assert!(valid.is_ok());
        Ok(())
    }

    fn ps_change_tests<P: ProfileTrait>(e: ProfileSet<P>) -> ockam_core::Result<()> {
        // change_events update_no_verification verify
        Ok(())
    }

    async fn ps_contacts_tests<P: ProfileTrait>(
        ctx: &Context,
        mut e: ProfileSet<P>,
    ) -> ockam_core::Result<()> {
        let alice = new_ps(&ctx).await.unwrap();
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

        let change_events = vec![];
        e.verify_and_update_contact(&alice_id, change_events)?;
        Ok(())
    }

    fn ps_secrets_tests<P: ProfileTrait>(mut e: ProfileSet<P>) -> ockam_core::Result<()> {
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

    async fn ps_profile_mgmt_test<P: ProfileTrait>(
        ctx: &Context,
        e: ProfileSet<P>,
    ) -> ockam_core::Result<()> {
        let vault = Vault::create(ctx)?;
        let bank_profile = Profile::create(ctx, &vault).await?;

        //    e.add_profile(bank_profile);
        Ok(())
    }

    async fn ps_all_tests(mut ctx: Context) -> ockam_core::Result<()> {
        let e = new_ps(&ctx).await?;
        ps_contacts_tests(&ctx, e.clone()).await?;
        ps_auth_tests(e.clone())?;
        ps_change_tests(e.clone())?;
        ps_secrets_tests(e.clone())?;
        ps_profile_mgmt_test(&ctx, e).await?;
        ctx.stop().await
    }

    #[test]
    fn test_ps_default_profile_delegation() {
        let (ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move { ps_all_tests(ctx).await })
            .unwrap();
    }
}
