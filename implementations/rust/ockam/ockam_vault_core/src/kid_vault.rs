use crate::secret::Secret;
use crate::{Kid, PublicKey};
use zeroize::Zeroize;

/// Key id related vault functionality
pub trait KidVault: Zeroize {
    /// Return [`Secret`] for given key id
    fn get_secret_by_kid(&self, kid: &str) -> ockam_core::Result<Secret>;
    /// Return Kid for given public key
    fn compute_kid_for_public_key(&self, public_key: &PublicKey) -> ockam_core::Result<Kid>;
}
