use crate::state::State;
use crate::{Initiator, Responder, XXVault};
use ockam_core::{async_trait, compat::{boxed::Box, sync::Arc}};
use ockam_core::{AsyncTryClone, Result};
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(AsyncTryClone)]
pub struct XXNewKeyExchanger<V: XXVault> {
    vault: Arc<V>,
}

impl<V: XXVault> XXNewKeyExchanger<V> {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: Arc<V>) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl<V: XXVault> NewKeyExchanger for XXNewKeyExchanger<V> {
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;
    /// Create a new initiator using the provided backing vault
    async fn initiator(&self) -> Result<Initiator<V>> {
        let ss = State::new(self.vault.clone())?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    async fn responder(&self) -> Result<Responder<V>> {
        let ss = State::new(self.vault.clone())?;
        Ok(Responder::new(ss))
    }
}
