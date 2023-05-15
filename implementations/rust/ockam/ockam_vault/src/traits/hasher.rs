use crate::{KeyId, SecretAttributes, SmallBuffer};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// A trait for hashing input data into a fixed length output.
#[async_trait]
pub trait Hasher: Sync + Send {
    /// Compute the SHA-256 digest given input `data`.
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]>;
    /// Derive multiple output [`super::Secret`]s with given attributes using
    /// the HKDF-SHA256 given the specified salt, info and input key
    /// material.
    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<KeyId>>;
}
