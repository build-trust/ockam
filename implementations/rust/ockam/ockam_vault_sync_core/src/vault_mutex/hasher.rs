use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{Hasher, Secret, SecretAttributes, SmallBuffer};

impl<V: Hasher> Hasher for VaultMutex<V> {
    fn sha256(&mut self, data: &[u8]) -> Result<[u8; 32]> {
        self.0.lock().unwrap().sha256(data)
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>> {
        self.0
            .lock()
            .unwrap()
            .hkdf_sha256(salt, info, ikm, output_attributes)
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test]
    fn sha256() {}

    #[vault_test]
    fn hkdf() {}
}
