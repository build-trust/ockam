use std::sync::{Arc, Mutex};
use tracing::debug;
use zeroize::Zeroize;

/// Vault inside Arc Mutex
pub struct VaultMutex<V>(Arc<Mutex<V>>);

impl<V> Clone for VaultMutex<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V: Zeroize> Zeroize for VaultMutex<V> {
    fn zeroize(&mut self) {
        self.0.lock().unwrap().zeroize()
    }
}

impl<V> VaultMutex<V> {
    /// Create and start a new Vault using Mutex.
    pub fn create(vault: V) -> Self {
        debug!("Starting VaultMutex");

        Self(Arc::new(Mutex::new(vault)))
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
