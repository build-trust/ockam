use crate::secret::Secret;
use crate::types::SecretAttributes;
use zeroize::Zeroize;

/// Hashing-related vault functionality
pub trait HashVault: Zeroize {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> ockam_core::Result<[u8; 32]>;
    /// Derive multiple output [`Secret`]s with given attributes using the HKDF-SHA256 using
    /// specified salt, input key material and info.
    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> ockam_core::Result<Vec<Secret>>;
}
