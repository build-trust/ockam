#[cfg(not(feature = "std"))]
use ockam_node::interrupt::Mutex;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};

use tracing::debug;
use zeroize::Zeroize;

/// Vault inside Arc Mutex
#[cfg(feature = "std")]
pub struct VaultMutex<V>(Arc<Mutex<V>>);

/// Vault inside Mutex RefCell Option (no_std)
#[cfg(not(feature = "std"))]
pub struct VaultMutex<V>(Mutex<RefCell<Option<V>>>);
#[cfg(not(feature = "std"))]
use core::cell::RefCell;

#[cfg(feature = "std")]
impl<V> Clone for VaultMutex<V> {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}
#[cfg(not(feature = "std"))]
impl<V: Clone> Clone for VaultMutex<V> {
    fn clone(&self) -> Self {
        // TODO @antoinevg - use new no_std mutex wrapper
        return ockam_node::interrupt::free(|cs| {
            let clone = self.0.borrow(cs).borrow_mut().clone();
            Self(Mutex::new(RefCell::new(clone)))
        });
    }
}

impl<V: Zeroize> Zeroize for VaultMutex<V> {
    fn zeroize(&mut self) {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().zeroize();
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0.borrow(cs).borrow_mut().as_mut().unwrap().zeroize()
        });
    }
}

impl<V> VaultMutex<V> {
    /// Create and start a new Vault using Mutex.
    pub fn create(vault: V) -> Self {
        debug!("Starting VaultMutex");

        #[cfg(feature = "std")]
        return Self(Arc::new(Mutex::new(vault)));
        #[cfg(not(feature = "std"))]
        return Self(Mutex::new(RefCell::new(Some(vault))));
    }
}

mod asymmetric_vault;
mod hasher;
mod key_id_vault;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod verifier;

pub use asymmetric_vault::*;
pub use hasher::*;
pub use key_id_vault::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use verifier::*;
