use crate::secret::Secret;
use crate::types::SecretAttributes;
use zeroize::Zeroize;

/// Vault with hashing functionality
pub trait HashVault: Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> ockam_core::Result<[u8; 32]>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> ockam_core::Result<Vec<Secret>>;
}
