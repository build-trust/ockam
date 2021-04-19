use crate::{Initiator, Responder, X3dhVault};
use ockam_key_exchange_core::NewKeyExchanger;
use std::sync::{Arc, Mutex};

/// Represents an XX NewKeyExchanger
pub struct X3dhNewKeyExchanger {
    vault_initiator: Arc<Mutex<dyn X3dhVault>>,
    vault_responder: Arc<Mutex<dyn X3dhVault>>,
}

impl std::fmt::Debug for X3dhNewKeyExchanger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "X3dhNewKeyExchanger {{ vault_initiator, vault_responder }}"
        )
    }
}

impl X3dhNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(
        vault_initiator: Arc<Mutex<dyn X3dhVault>>,
        vault_responder: Arc<Mutex<dyn X3dhVault>>,
    ) -> Self {
        Self {
            vault_initiator,
            vault_responder,
        }
    }
}

impl NewKeyExchanger for X3dhNewKeyExchanger {
    type Initiator = Initiator;
    type Responder = Responder;

    fn initiator(&self) -> ockam_core::Result<Initiator> {
        Ok(Initiator::new(self.vault_initiator.clone(), None))
    }

    fn responder(&self) -> ockam_core::Result<Responder> {
        Ok(Responder::new(self.vault_responder.clone(), None))
    }
}
