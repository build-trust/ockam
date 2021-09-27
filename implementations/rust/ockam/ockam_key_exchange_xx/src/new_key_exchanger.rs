use crate::state::State;
use crate::{Initiator, Responder, XXVault};
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(Clone)]
pub struct XXNewKeyExchanger<V: XXVault> {
    vault: V,
}

impl<V: XXVault> XXNewKeyExchanger<V> {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: V) -> Self {
        Self { vault }
    }
}

use ockam_core::async_trait::async_trait;
#[async_trait]
impl<V: XXVault + Sync> NewKeyExchanger for XXNewKeyExchanger<V> {
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;
    /// Create a new initiator using the provided backing vault
    fn initiator(&self) -> Result<Initiator<V>> {
        let ss = State::new(&self.vault)?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    fn responder(&self) -> Result<Responder<V>> {
        let ss = State::new(&self.vault)?;
        Ok(Responder::new(ss))
    }

    /// Create a new initiator using the provided backing vault
    async fn async_initiator(&self) -> Result<Initiator<V>> {
        let ss = State::async_new(&self.vault).await?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    async fn async_responder(&self) -> Result<Responder<V>> {
        let ss = State::async_new(&self.vault).await?;
        Ok(Responder::new(ss))
    }
}
