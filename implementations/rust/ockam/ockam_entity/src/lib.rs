//! Entity is an abstraction over Profiles and Vaults, easing the use of these primitives in
//! authentication and authorization APIs.
#![deny(
  // prevented by big_array
  //  missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    // TODO re-enable warnings
)]

use crate::EntityError;
use ockam_channel::SecureChannelVault;
use ockam_core::lib::HashMap;
use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_vault_core::{Hasher, KeyIdVault, SecretVault, Signer, Verifier};
use ockam_vault_sync_core::VaultSync;

mod imp;
pub use imp::*;
mod traits;
pub use traits::*;
mod authentication;
mod contact;
pub use contact::*;
mod identifiers;
pub use identifiers::*;
mod key_attributes;
pub use key_attributes::*;
mod change;
pub use change::*;
mod channel;
pub(crate) use channel::*;
mod entity;
pub use entity::*;
mod error;
pub use error::*;
mod worker;
pub use worker::*;

/// Traits required for a Vault implementation suitable for use in a Profile
pub trait ProfileVault:
    SecretVault + SecureChannelVault + KeyIdVault + Hasher + Signer + Verifier + Clone + Send + 'static
{
}

impl<D> ProfileVault for D where
    D: SecretVault
        + SecureChannelVault
        + KeyIdVault
        + Hasher
        + Signer
        + Verifier
        + Clone
        + Send
        + 'static
{
}

/// Profile is an abstraction responsible for keeping, verifying and modifying
/// user's data (mainly - public keys). It is used to create new keys, rotate and revoke them.
/// Public keys together with metadata will be organised into events chain, corresponding
/// secret keys will be saved into the given Vault implementation. Events chain and corresponding
/// secret keys are what fully determines Profile.
///
///
/// # Examples
///
/// Create a [`Profile`]. Add and rotate keys.
///
/// ```
/// # use ockam_core::Result;
/// # use ockam_vault::SoftwareVault;
/// # use ockam_vault_sync_core::Vault;
/// # use ockam_entity::{Profile, KeyAttributes, ProfileSecrets, ProfileChanges};
/// # fn main() -> Result<()> {
/// # let (mut ctx, mut executor) = ockam_node::start_node();
/// # executor.execute(async move {
/// let vault = Vault::create(&ctx)?;
/// let mut profile = Profile::create(&ctx, &vault).await?;
///
/// let root_key_attributes = KeyAttributes::new(
///     Profile::PROFILE_UPDATE.to_string(),
/// );
///
/// let _alice_root_secret = profile.get_secret_key(&root_key_attributes)?;
///
/// let truck_key_attributes = KeyAttributes::new(
///     "Truck management".to_string(),
/// );
///
/// profile.create_key(truck_key_attributes.clone(), None)?;
///
/// let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes)?;
///
/// profile.rotate_key(truck_key_attributes.clone(), None)?;
///
/// let alice_truck_secret = profile.get_secret_key(&truck_key_attributes)?;
///
/// let verified = profile.verify()?;
/// # ctx.stop().await.unwrap();
/// # Ok::<(), ockam_core::Error>(())
/// # }).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// Authentication using [`Profile`]. In following example Bob authenticates Alice.
///
/// ```
/// # use ockam_core::Result;
/// # use ockam_vault::SoftwareVault;
/// # use ockam_vault_sync_core::Vault;
/// # use ockam_entity::{Profile, ProfileAuth, ProfileContacts};
/// fn alice_main() -> Result<()> {
///     # let (mut ctx, mut executor) = ockam_node::start_node();
///     # executor.execute(async move {
///     let vault = Vault::create(&ctx)?;
///
///     // Alice generates profile
///     let mut alice = Profile::create(&ctx, &vault).await?;
///
///     // Key agreement happens here
///     let key_agreement_hash = [0u8; 32];
///
///     // Send this over the network to Bob
///     let contact_alice = alice.serialize_to_contact()?;
///     let proof_alice = alice.generate_authentication_proof(&key_agreement_hash)?;
///     # ctx.stop().await.unwrap();
///     # Ok::<(), ockam_core::Error>(())
///     # }).unwrap();
///     Ok(())
/// }
///
/// fn bob_main() -> Result<()> {
///     # let (mut ctx, mut executor) = ockam_node::start_node();
///     # executor.execute(async move {
///     let vault = Vault::create(&ctx)?;
///
///     // Bob generates profile
///     let mut bob = Profile::create(&ctx, &vault).await?;
///
///     // Key agreement happens here
///     let key_agreement_hash = [0u8; 32];
///
///     // Receive this from Alice over the network
///     # let contact_alice = [0u8; 32];
///     let contact_alice = Profile::deserialize_contact(&contact_alice)?;
///     let alice_id = contact_alice.identifier().clone();
///
///     // Bob adds Alice to contact list
///     bob.verify_and_add_contact(contact_alice)?;
///
///     # let proof_alice = [0u8; 32];
///     // Bob verifies Alice
///     let verified = bob.verify_authentication_proof(&key_agreement_hash, &alice_id, &proof_alice)?;
///     # ctx.stop().await.unwrap();
///     # Ok::<(), ockam_core::Error>(())
///     # }).unwrap();
///     Ok(())
/// }
/// ```
///
/// Update [`Profile`] and send changes to other parties. In following example Alice rotates
/// her key and sends corresponding [`Profile`] changes to Bob.
///
/// ```
/// # use ockam_core::Result;
/// # use ockam_vault::SoftwareVault;
/// # use ockam_vault_sync_core::Vault;
/// # use ockam_entity::{Profile, ProfileContacts, ProfileChanges, ProfileSecrets};
/// fn alice_main() -> Result<()> {
///     # let (mut ctx, mut executor) = ockam_node::start_node();
///     # executor.execute(async move {
///     # let vault = Vault::create(&ctx)?;
///     # let mut alice = Profile::create(&ctx, &vault).await?;
///     # let key_agreement_hash = [0u8; 32];
///     # let contact_alice = alice.serialize_to_contact()?;
///     #
///     let index_a = alice.change_events()?.len();
///     alice.rotate_key(Profile::PROFILE_UPDATE.into(), None)?;
///
///     // Send to Bob
///     let change_events = &alice.change_events()?[index_a..];
///     let change_events = Profile::serialize_change_events(change_events)?;
///     # ctx.stop().await.unwrap();
///     # Ok::<(), ockam_core::Error>(())
///     # }).unwrap();
///     Ok(())
/// }
///
/// fn bob_main() -> Result<()> {
///     # let (mut ctx, mut executor) = ockam_node::start_node();
///     # executor.execute(async move {
///     # let vault = Vault::create(&ctx)?;
///     # let mut bob = Profile::create(&ctx, &vault).await?;
///     # let key_agreement_hash = [0u8; 32];
///     # let contact_alice = [0u8; 32];
///     # let contact_alice = Profile::deserialize_contact(&contact_alice)?;
///     # let alice_id = contact_alice.identifier().clone();
///     # bob.verify_and_add_contact(contact_alice)?;
///     // Receive from Alice
///     # let change_events = [0u8; 32];
///     let change_events = Profile::deserialize_change_events(&change_events)?;
///     bob.verify_and_update_contact(&alice_id, change_events)?;
///     # ctx.stop().await.unwrap();
///     # Ok::<(), ockam_core::Error>(())
///     # }).unwrap();
///     Ok(())
/// }
/// ```
pub struct Profile;

impl Profile {
    /// Create a new Profile
    pub async fn create(ctx: &Context, vault: &Address) -> Result<ProfileSync> {
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        let imp = ProfileImpl::<VaultSync>::create_internal(None, vault)?;
        ProfileSync::create(ctx, imp).await
    }
}

impl Profile {
    /// Sha256 of that value is used as previous event id for first event in a [`Profile`]
    pub const NO_EVENT: &'static [u8] = "OCKAM_NO_EVENT".as_bytes();
    /// Label for [`Profile`] update key
    pub const PROFILE_UPDATE: &'static str = "OCKAM_PUK";
    /// Label for key used to issue credentials
    pub const CREDENTIALS_ISSUE: &'static str = "OCKAM_CIK";
    /// Current version of change structure
    pub const CURRENT_CHANGE_VERSION: u8 = 1;
}

/// Profile event attributes
pub type ProfileEventAttributes = HashMap<String, String>;
/// Contacts Database
pub type ContactsDb = HashMap<ProfileIdentifier, Contact>;

impl Profile {
    /// Serialize [`Contact`] in binary form for storing/transferring over the network
    pub fn serialize_contact(contact: &Contact) -> Result<Vec<u8>> {
        serde_bare::to_vec(&contact).map_err(|_| EntityError::BareError.into())
    }

    /// Deserialize [`Contact`] from binary form
    pub fn deserialize_contact(contact: &[u8]) -> Result<Contact> {
        let contact: Contact =
            serde_bare::from_slice(contact).map_err(|_| EntityError::BareError)?;

        Ok(contact)
    }

    /// Serialize [`ProfileChangeEvent`]s to binary form for storing/transferring over the network
    pub fn serialize_change_events(change_events: &[ProfileChangeEvent]) -> Result<Vec<u8>> {
        serde_bare::to_vec(&change_events).map_err(|_| EntityError::BareError.into())
    }

    /// Deserialize [`ProfileChangeEvent`]s from binary form
    pub fn deserialize_change_events(change_events: &[u8]) -> Result<Vec<ProfileChangeEvent>> {
        let change_events: Vec<ProfileChangeEvent> =
            serde_bare::from_slice(change_events).map_err(|_| EntityError::BareError)?;

        Ok(change_events)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use ockam_vault_sync_core::Vault;

    fn fn_test_new<P: ProfileTrait>(profile: &mut P) {
        profile.verify().unwrap();

        let root_key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());

        let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();
        let _alice_root_public_key = profile.get_public_key(&root_key_attributes).unwrap();

        let truck_key_attributes = KeyAttributes::new("Truck management".to_string());

        profile
            .create_key(truck_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
        let _alice_truck_public_key = profile.get_public_key(&truck_key_attributes).unwrap();

        profile
            .rotate_key(truck_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
        let _alice_truck_public_key = profile.get_public_key(&truck_key_attributes).unwrap();

        profile
            .rotate_key(root_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();
        let _alice_root_public_key = profile.get_public_key(&root_key_attributes).unwrap();
    }

    #[test]
    fn test_new() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create(&ctx).unwrap();
                let mut profile = Profile::create(&ctx, &vault).await.unwrap();
                fn_test_new(&mut profile);

                ctx.stop().await.unwrap();
            })
            .unwrap();
    }

    fn fn_test_update<P: ProfileTrait>(alice: &mut P, bob: &mut P) {
        // Receive this from Alice over the network
        let contact_alice = alice.serialize_to_contact().unwrap();
        let contact_alice = Profile::deserialize_contact(&contact_alice).unwrap();
        let alice_id = contact_alice.identifier().clone();
        // Bob adds Alice to contact list
        bob.verify_and_add_contact(contact_alice).unwrap();

        alice
            .rotate_key(Profile::PROFILE_UPDATE.into(), None)
            .unwrap();

        let index_a = alice.change_events().unwrap().len();
        let change_events = &alice.change_events().unwrap()[index_a..];
        let change_events = Profile::serialize_change_events(change_events).unwrap();

        // Receive from Alice
        let change_events = Profile::deserialize_change_events(&change_events).unwrap();
        assert!(bob
            .verify_and_update_contact(&alice_id, change_events)
            .unwrap());
    }

    #[test]
    fn test_update() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create(&ctx).unwrap();
                let mut alice = Profile::create(&ctx, &vault).await.unwrap();
                let mut bob = Profile::create(&ctx, &vault).await.unwrap();
                fn_test_update(&mut alice, &mut bob);

                ctx.stop().await.unwrap();
            })
            .unwrap();
    }
}
