use crate::state::State;
use crate::{Initiator, Responder, XXVault};
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

impl<V: XXVault> NewKeyExchanger for XXNewKeyExchanger<V> {
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
}
