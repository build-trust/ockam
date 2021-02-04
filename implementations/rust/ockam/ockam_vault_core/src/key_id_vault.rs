use crate::secret::Secret;
use crate::{KeyId, PublicKey};
use zeroize::Zeroize;

/// Key id related vault functionality
pub trait KeyIdVault: Zeroize {
    /// Return [`Secret`] for given key id
    fn get_secret_by_key_id(&self, key_id: &str) -> ockam_core::Result<Secret>;
    /// Return KeyId for given public key
    fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> ockam_core::Result<KeyId>;
}
