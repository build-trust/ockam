use crate::vault::{KeyId, PublicKey, Secret};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Key id related vault functionality
#[async_trait]
pub trait KeyIdVault {
    /// Return [`Secret`] for given key id
    async fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret>;
    /// Return KeyId for given public key
    async fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId>;
}
