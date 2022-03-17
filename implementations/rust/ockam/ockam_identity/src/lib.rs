//! Identity is an abstraction over Identitys and Vaults, easing the use of these primitives in
//! authentication and authorization APIs.
#![deny(unsafe_code)]
#![warn(
    // prevented by big_array
    //  missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

use cfg_if::cfg_if;

pub use change::*;
pub use channel::*;
pub use contact::*;
pub use error::*;
pub use identifiers::*;
pub use identity::*;
pub use identity_builder::*;
pub use identity_state::*;
pub use key_attributes::*;
pub use lease::*;
use ockam_channel::SecureChannelVault;
use ockam_core::compat::{collections::HashMap, string::String, vec::Vec};
use ockam_core::{AsyncTryClone, Decodable, Encodable, Result};
use ockam_vault::{Hasher, KeyIdVault, SecretVault, Signer, Verifier};
pub use traits::*;
pub use worker::*;

use crate::IdentityError;

mod authentication;
mod change;
pub mod change_history;
mod channel;
mod contact;
mod error;
mod identifiers;
mod identity;
mod identity_builder;
mod identity_state;
mod key_attributes;
mod lease;
mod signature;
mod traits;
mod worker;

cfg_if! {
    if #[cfg(feature = "credentials")] {
        mod credential;
        pub use credential::*;
    }
}

/// Traits required for a Vault implementation suitable for use in an Identity
pub trait IdentityVault:
    SecretVault
    + SecureChannelVault
    + KeyIdVault
    + Hasher
    + Signer
    + Verifier
    + AsyncTryClone
    + Send
    + 'static
{
}

impl<D> IdentityVault for D where
    D: SecretVault
        + SecureChannelVault
        + KeyIdVault
        + Hasher
        + Signer
        + Verifier
        + AsyncTryClone
        + Send
        + 'static
{
}

/// Identity event attributes
pub type IdentityEventAttributes = HashMap<String, String>;
/// Contacts Database
pub type Contacts = HashMap<IdentityIdentifier, Contact>;

#[cfg(feature = "credentials")]
pub use signature_bbs_plus::{PublicKey as BbsPublicKey, SecretKey as BbsSecretKey};
#[cfg(feature = "credentials")]
pub use signature_bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};

pub struct IdentitySerializationUtil;

impl IdentitySerializationUtil {
    /// Serialize [`Contact`] in binary form for storing/transferring over the network
    pub fn serialize_contact(contact: &Contact) -> Result<Vec<u8>> {
        contact
            .encode()
            .map_err(|_| IdentityError::BareError.into())
    }

    /// Deserialize [`Contact`] from binary form
    pub fn deserialize_contact(contact: &[u8]) -> Result<Contact> {
        let contact = Contact::decode(contact).map_err(|_| IdentityError::BareError)?;

        Ok(contact)
    }

    /// Serialize [`IdentityChangeEvent`]s to binary form for storing/transferring over the network
    pub fn serialize_change_events(change_events: &[IdentityChangeEvent]) -> Result<Vec<u8>> {
        change_events
            .encode()
            .map_err(|_| IdentityError::BareError.into())
    }

    /// Deserialize [`IdentityChangeEvent`]s from binary form
    pub fn deserialize_change_events(change_events: &[u8]) -> Result<Vec<IdentityChangeEvent>> {
        let change_events = Vec::<IdentityChangeEvent>::decode(change_events)
            .map_err(|_| IdentityError::BareError)?;

        Ok(change_events)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_core::Error;
    use ockam_node::Context;
    use ockam_vault::Vault;

    fn test_error<S: Into<String>>(msg: S) -> Result<()> {
        Err(Error::new(0, msg.into()))
    }

    async fn test_basic_identity_key_ops(identity: &mut (impl IdentityTrait + Sync)) -> Result<()> {
        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret1 = identity.get_root_secret_key().await?;
        let public1 = identity.get_root_public_key().await?;

        identity.create_key("Truck management".to_string()).await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret2 = identity
            .get_secret_key("Truck management".to_string())
            .await?;
        let public2 = identity.get_public_key("Truck management".into()).await?;

        if secret1 == secret2 {
            return test_error("secret did not change after create_key");
        }

        if public1 == public2 {
            return test_error("public did not change after create_key");
        }

        identity.rotate_root_secret_key().await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret3 = identity.get_root_secret_key().await?;
        let public3 = identity.get_root_public_key().await?;

        identity.rotate_root_secret_key().await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        if secret1 == secret3 {
            return test_error("secret did not change after rotate_key");
        }

        if public1 == public3 {
            return test_error("public did not change after rotate_key");
        }

        Ok(())
    }

    async fn test_update_contact_after_change(
        alice: &mut (impl IdentityTrait + Sync),
        bob: &mut (impl IdentityTrait + Sync),
    ) -> Result<()> {
        let contact_alice = alice.as_contact().await?;
        let alice_id = contact_alice.identifier().clone();
        if !bob.verify_and_add_contact(contact_alice).await? {
            return test_error("bob failed to add alice");
        }

        alice.rotate_root_secret_key().await?;
        let alice_changes = alice.get_changes().await?;
        let last_change = alice_changes.last().unwrap().clone();

        if !bob
            .verify_and_update_contact(&alice_id, &[last_change])
            .await?
        {
            return test_error("bob failed to update alice");
        }
        Ok(())
    }

    #[ockam_macros::test]
    async fn async_tests(ctx: &mut Context) -> Result<()> {
        let alice_vault = Vault::create();
        let bob_vault = Vault::create();

        let mut alice = Identity::create(ctx, &alice_vault).await?;
        let mut bob = Identity::create(ctx, &bob_vault).await?;

        let mut results = vec![];
        results.push(test_basic_identity_key_ops(&mut alice).await);
        results.push(test_update_contact_after_change(&mut alice, &mut bob).await);
        ctx.stop().await?;

        for r in results {
            match r {
                Err(e) => panic!("{}", e.domain().clone()),
                _ => (),
            }
        }
        Ok(())
    }
}
