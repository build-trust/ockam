use crate::secret::Secret;
use crate::{KeyId, PublicKey};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

/// Key id related vault functionality
#[async_trait]
pub trait KeyIdVault: Send + Sync {
    /// Return [`Secret`] for given key id
    async fn get_secret_by_key_id(&self, key_id: &str) -> Result<Secret>;
    /// Return KeyId for given public key
    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId>;
}

#[async_trait]
impl<V: ?Sized + KeyIdVault> KeyIdVault for ockam_core::compat::sync::Arc<V> {
    async fn get_secret_by_key_id(&self, key_id: &str) -> Result<Secret> {
        V::get_secret_by_key_id(&**self, key_id).await
    }
    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
        V::compute_key_id_for_public_key(&**self, public_key).await
    }
}
