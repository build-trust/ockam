use hashbrown::HashMap;
use ockam_core::Error;
use ockam_macro::zdrop_impl;
use ockam_vault_core::hash_vault::HashVault;
use ockam_vault_core::open_close_vault::OpenCloseVault;
use ockam_vault_core::types::{SecretAttributes, SecretKey};
use ockam_vault_core::vault::Vault;
use sha2::Digest;
use zeroize::Zeroize;

#[derive(Zeroize, Debug, Eq, PartialEq)]
struct VaultEntry {
    id: usize,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

zdrop_impl!(VaultEntry);

pub struct SoftwareVault {
    _entries: HashMap<usize, VaultEntry>,
    next_id: usize,
}

zdrop_impl!(SoftwareVault);

impl Zeroize for SoftwareVault {
    fn zeroize(&mut self) {
        self.next_id.zeroize();
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self {
            _entries: Default::default(),
            next_id: 0,
        }
    }
}

impl OpenCloseVault for SoftwareVault {
    fn open(&mut self) -> Result<Vault<'_, Self>, Error> {
        Ok(Vault::new(self))
    }

    fn close(&mut self) {}
}

impl HashVault for SoftwareVault {
    fn sha256(&self, msg: &[u8]) -> Result<[u8; 32], Error> {
        Ok(sha2::Sha256::digest(msg).into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut vault = SoftwareVault::default();
        let vault = vault.open().unwrap();

        let hash = vault.sha256(&[0u8; 32]).unwrap();
        assert_eq!(
            hex::encode(hash),
            "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
        );
    }
}
