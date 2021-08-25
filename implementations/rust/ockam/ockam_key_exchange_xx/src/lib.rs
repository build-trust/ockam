//! XX (Noise Protocol) implementation of an Ockam Key Exchanger.
//!
//! This crate contains the key exchange types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

mod error;
pub use error::*;

/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE: usize = 16;

/// Vault with XX required functionality
pub trait XXVault:
    SecretVault + Hasher + AsymmetricVault + SymmetricVault + Clone + Send + 'static
{
}

impl<D> XXVault for D where
    D: SecretVault + Hasher + AsymmetricVault + SymmetricVault + Clone + Send + 'static
{
}

mod initiator;
mod state;
pub use initiator::*;
mod responder;
pub use responder::*;
mod new_key_exchanger;
pub use new_key_exchanger::*;
use ockam_vault_core::{AsymmetricVault, Hasher, SecretVault, SymmetricVault};

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_key_exchange_core::{KeyExchanger, NewKeyExchanger};
    use ockam_vault::SoftwareVault;
    use ockam_vault_core::SecretVault;
    use ockam_vault_sync_core::VaultMutex;

    #[allow(non_snake_case)]
    #[test]
    fn full_flow__correct_credentials__keys_should_match() {
        let mut vault = VaultMutex::create(SoftwareVault::default());

        let key_exchanger = XXNewKeyExchanger::new(vault.clone());

        let mut initiator = key_exchanger.initiator().unwrap();
        let mut responder = key_exchanger.responder().unwrap();

        loop {
            if !initiator.is_complete() {
                let m = initiator.generate_request(&[]).unwrap();
                let _ = responder.handle_response(&m).unwrap();
            }

            if !responder.is_complete() {
                let m = responder.generate_request(&[]).unwrap();
                let _ = initiator.handle_response(&m).unwrap();
            }

            if initiator.is_complete() && responder.is_complete() {
                break;
            }
        }

        let initiator = initiator.finalize().unwrap();
        let responder = responder.finalize().unwrap();

        assert_eq!(initiator.h(), responder.h());

        let s1 = vault.secret_export(&initiator.encrypt_key()).unwrap();
        let s2 = vault.secret_export(&responder.decrypt_key()).unwrap();

        assert_eq!(s1, s2);

        let s1 = vault.secret_export(&initiator.decrypt_key()).unwrap();
        let s2 = vault.secret_export(&responder.encrypt_key()).unwrap();

        assert_eq!(s1, s2);
    }
}
