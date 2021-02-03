use crate::secret::Secret;
use ockam_core::Error;
use zeroize::Zeroize;

pub trait KidVault: Zeroize {
    fn get_secret_by_kid(&self, kid: &str) -> Result<Secret, Error>;
}
