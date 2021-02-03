use crate::VaultError;
use ockam_vault_core::zdrop_impl;
use ockam_vault_core::{Secret, SecretAttributes, SecretKey};
use std::collections::BTreeMap;
use zeroize::Zeroize;

/// Vault implementation that stores secrets in memory and uses software crypto.
#[derive(Debug)]
pub struct SoftwareVault {
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    pub(crate) next_id: usize,
}

impl SoftwareVault {
    pub fn new() -> Self {
        Self {
            entries: Default::default(),
            next_id: 0,
        }
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareVault {
    pub(crate) fn get_entry(&self, context: &Secret) -> ockam_core::Result<&VaultEntry> {
        self.entries
            .get(&context.index())
            .ok_or_else(|| VaultError::EntryNotFound.into())
    }
}

impl Zeroize for SoftwareVault {
    fn zeroize(&mut self) {
        for (_, v) in self.entries.iter_mut() {
            v.zeroize();
        }
        self.entries.clear();
        self.next_id = 0;
    }
}

zdrop_impl!(SoftwareVault);

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VaultEntry {
    kid: Option<String>,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl VaultEntry {
    pub fn kid(&self) -> &Option<String> {
        &self.kid
    }
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }
    pub fn key(&self) -> &SecretKey {
        &self.key
    }
}

impl VaultEntry {
    pub fn new(kid: Option<String>, key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            kid,
            key_attributes,
            key,
        }
    }
}

impl Zeroize for VaultEntry {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

zdrop_impl!(VaultEntry);

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;

    #[test]
    fn new_vault() {
        let vault = SoftwareVault::new();
        assert_eq!(vault.next_id, 0);
        assert_eq!(vault.entries.len(), 0);
    }
}
