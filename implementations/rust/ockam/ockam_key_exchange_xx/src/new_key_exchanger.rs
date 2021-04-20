use crate::state::State;
use crate::{Initiator, Responder};
use ockam_core::Result;
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_vault_sync_core::VaultSync;

/// Represents an XX NewKeyExchanger
pub struct XXNewKeyExchanger {
    vault: VaultSync,
}

impl XXNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: VaultSync) -> Self {
        Self { vault }
    }
}

impl NewKeyExchanger for XXNewKeyExchanger {
    type Initiator = Initiator;
    type Responder = Responder;
    /// Create a new initiator using the provided backing vault
    fn initiator(&self) -> Result<Initiator> {
        let ss = State::new(&self.vault)?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    fn responder(&self) -> Result<Responder> {
        let ss = State::new(&self.vault)?;
        Ok(Responder::new(ss))
    }
}
