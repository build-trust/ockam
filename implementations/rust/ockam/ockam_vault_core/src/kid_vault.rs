use crate::secret::Secret;
use zeroize::Zeroize;

/// Key id related vault functionality
pub trait KidVault: Zeroize {
    /// Return [`Secret`] for given key id
    fn get_secret_by_kid(&self, kid: &str) -> ockam_core::Result<Secret>;
}
