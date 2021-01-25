use crate::open_close_vault::OpenCloseVault;
use crate::vault::Vault;
use ockam_core::Error;
use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

/// Vault with hashing functionality
pub trait HashVault: OpenCloseVault + Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], Error>;
}

impl<'a, T> Deref for Vault<'a, T>
where
    T: HashVault,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.inner
    }
}

impl<'a, T> DerefMut for Vault<'a, T>
where
    T: HashVault,
{
    fn deref_mut(&mut self) -> &mut T {
        self.inner
    }
}
