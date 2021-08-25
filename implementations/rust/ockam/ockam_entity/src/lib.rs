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
    // warnings
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

pub use change::*;
pub use channel::*;
pub use contact::*;
pub use credential::*;
pub use entity::*;
pub use entity_builder::*;
pub use error::*;
pub use identifiers::*;
pub use key_attributes::*;
pub use lease::*;
use ockam_channel::SecureChannelVault;
use ockam_core::compat::{collections::HashMap, string::String, vec::Vec};
use ockam_core::{Address, Message, Result};
use ockam_node::{block_future, Context};
use ockam_vault::{Hasher, KeyIdVault, SecretVault, Signer, Verifier};
pub use profile::*;
pub use profile_state::*;
pub use traits::*;
pub use worker::*;

use crate::EntityError;

pub struct Handle {
    ctx: Context,
    address: Address,
}

impl Clone for Handle {
    fn clone(&self) -> Self {
        block_future(&self.ctx.runtime(), async move {
            Handle {
                ctx: self
                    .ctx
                    .new_context(Address::random(0))
                    .await
                    .expect("new_context failed"),
                address: self.address.clone(),
            }
        })
    }
}

impl Handle {
    pub fn new(ctx: Context, address: Address) -> Self {
        Handle { ctx, address }
    }

    pub async fn async_cast<M: Message + Send + 'static>(&self, msg: M) -> Result<()> {
        self.ctx.send(self.address.clone(), msg).await
    }

    pub fn cast<M: Message + Send + 'static>(&self, msg: M) -> Result<()> {
        block_future(
            &self.ctx.runtime(),
            async move { self.async_cast(msg).await },
        )
    }

    pub async fn async_call<I: Message + Send + 'static, O: Message + Send + 'static>(
        &self,
        msg: I,
    ) -> Result<O> {
        let mut ctx = self
            .ctx
            .new_context(Address::random(0))
            .await
            .expect("new_context failed");
        ctx.send(self.address.clone(), msg).await?;
        let msg = ctx.receive::<O>().await?;
        Ok(msg.take().body())
    }

    pub fn call<I: Message + Send + 'static, O: Message + Send + 'static>(
        &self,
        msg: I,
    ) -> Result<O> {
        block_future(
            &self.ctx.runtime(),
            async move { self.async_call(msg).await },
        )
    }
}

mod authentication;
mod change;
pub mod change_history;
mod channel;
mod contact;
mod credential;
mod entity;
mod entity_builder;
mod error;
mod identifiers;
mod key_attributes;
mod lease;
mod profile;
mod profile_state;
mod proof;
mod traits;
mod worker;

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

/// Profile event attributes
pub type ProfileEventAttributes = HashMap<String, String>;
/// Contacts Database
pub type Contacts = HashMap<ProfileIdentifier, Contact>;

pub use signature_bbs_plus::{PublicKey as BbsPublicKey, SecretKey as BbsSecretKey};
pub use signature_bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};

pub struct ProfileSerializationUtil;

impl ProfileSerializationUtil {
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
    use ockam_core::Error;
    use ockam_vault_sync_core::Vault;

    fn test_error<S: Into<String>>(msg: S) -> Result<()> {
        Err(Error::new(0, msg.into()))
    }

    fn test_basic_profile_key_ops<P: Identity>(profile: &mut P) -> Result<()> {
        if !profile.verify_changes()? {
            return test_error("verify_changes failed");
        }

        let secret1 = profile.get_profile_secret_key()?;
        let public1 = profile.get_profile_public_key()?;

        profile.create_key("Truck management")?;

        if !profile.verify_changes()? {
            return test_error("verify_changes failed");
        }

        let secret2 = profile.get_secret_key("Truck management")?;
        let public2 = profile.get_public_key("Truck management")?;

        if secret1 == secret2 {
            return test_error("secret did not change after create_key");
        }

        if public1 == public2 {
            return test_error("public did not change after create_key");
        }

        profile.rotate_profile_key()?;

        if !profile.verify_changes()? {
            return test_error("verify_changes failed");
        }

        let secret3 = profile.get_profile_secret_key()?;
        let public3 = profile.get_profile_public_key()?;

        profile.rotate_profile_key()?;

        if !profile.verify_changes()? {
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

    fn test_update_contact_after_change<P: Identity>(alice: &mut P, bob: &mut P) -> Result<()> {
        let contact_alice = alice.as_contact()?;
        let alice_id = contact_alice.identifier().clone();
        if !bob.verify_and_add_contact(contact_alice)? {
            return test_error("bob failed to add alice");
        }

        alice.rotate_profile_key()?;
        let alice_changes = alice.get_changes()?;
        let last_change = alice_changes.last().unwrap().clone();

        if !bob.verify_and_update_contact(&alice_id, &[last_change])? {
            return test_error("bob failed to update alice");
        }
        Ok(())
    }

    #[test]
    fn async_tests() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let alice_vault = Vault::create(&ctx).expect("failed to create vault");
                let bob_vault = Vault::create(&ctx).expect("failed to create vault");

                let mut entity_alice = Entity::create(&ctx, &alice_vault).unwrap();
                let mut entity_bob = Entity::create(&ctx, &bob_vault).unwrap();

                let mut alice = entity_alice.current_profile().unwrap();
                let mut bob = entity_bob.current_profile().unwrap();

                let mut results = vec![];
                results.push(test_basic_profile_key_ops(&mut alice));
                results.push(test_update_contact_after_change(&mut alice, &mut bob));
                ctx.stop().await.unwrap();

                for r in results {
                    match r {
                        Err(e) => panic!("{}", e.domain().clone()),
                        _ => (),
                    }
                }
            })
            .unwrap();
    }
}
