use crate::secret::Secret;
use crate::{KeyId, PublicKey};
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use zeroize::Zeroize;

use ockam_core::async_trait::async_trait;
#[async_trait]
/// Key id related vault functionality
pub trait KeyIdVault: Zeroize {
    /// Return [`Secret`] for given key id
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret>;
    /// Return [`Secret`] for given key id
    async fn async_get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret>;
    /// Return KeyId for given public key
    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId>;
    /// Return KeyId for given public key
    async fn async_compute_key_id_for_public_key(
        &mut self,
        public_key: &PublicKey,
    ) -> Result<KeyId>;
}
