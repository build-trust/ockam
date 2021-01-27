use crate::secret::Secret;
use crate::types::SecretAttributes;
use ockam_core::Error;
use zeroize::Zeroize;

/// Vault with hashing functionality
pub trait HashVault: Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], Error>;
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Secret>, Error>;
}
