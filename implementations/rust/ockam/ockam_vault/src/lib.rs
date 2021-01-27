use hashbrown::HashMap;
use ockam_macro::zdrop_impl;
use ockam_vault_core::types::{SecretAttributes, SecretKey};
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
