use crate::secret::Secret;
use zeroize::Zeroize;

pub trait KidVault: Zeroize {
    fn get_secret_by_kid(&self, kid: &str) -> ockam_core::Result<Secret>;
}
