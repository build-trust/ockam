use crate::state::State;
use crate::{Initiator, Responder, XXVault};
use ockam_core::{async_trait, compat::boxed::Box, AsyncTryClone, Result};

use ockam_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
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
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;

    /// Create a new initiator using the provided backing vault
    async fn initiator(&self) -> Result<Initiator<V>> {
        let ss = State::new(&self.vault).await?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    async fn responder(&self) -> Result<Responder<V>> {
        let ss = State::new(&self.vault).await?;
        Ok(Responder::new(ss))
    }
}
