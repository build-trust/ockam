use crate::{KeyExchange, XXVault};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(AsyncTryClone)]
pub struct XXNewKeyExchanger<V: XXVault> {
    vault: V,
}

impl<V: XXVault> XXNewKeyExchanger<V> {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: V) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl<V: XXVault> NewKeyExchanger for XXNewKeyExchanger<V> {
    type Initiator = KeyExchange<V>;
    type Responder = KeyExchange<V>;
    /// Create a new initiator using the provided backing vault
    async fn initiator(&self) -> Result<KeyExchange<V>> {
        KeyExchange::new(true, self.vault.async_try_clone().await?).await
    }

    /// Create a new responder using the provided backing vault
    async fn responder(&self) -> Result<KeyExchange<V>> {
        KeyExchange::new(false, self.vault.async_try_clone().await?).await
    }
}
