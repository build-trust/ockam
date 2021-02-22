use crate::state::State;
use crate::{Initiator, Responder, XXVault};
use ockam_key_exchange_core::NewKeyExchanger;
use std::sync::{Arc, Mutex};

/// Represents an XX NewKeyExchanger
pub struct XXNewKeyExchanger {
    vault_initiator: Arc<Mutex<dyn XXVault>>,
    vault_responder: Arc<Mutex<dyn XXVault>>,
}

impl XXNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(
        vault_initiator: Arc<Mutex<dyn XXVault>>,
        vault_responder: Arc<Mutex<dyn XXVault>>,
    ) -> Self {
        Self {
            vault_initiator,
            vault_responder,
        }
    }
}

impl NewKeyExchanger<Initiator, Responder> for XXNewKeyExchanger {
    /// Create a new initiator using the provided backing vault
    fn initiator(&self) -> Initiator {
        let ss = State::new(self.vault_initiator.clone());
        Initiator::new(ss)
    }

    /// Create a new responder using the provided backing vault
    fn responder(&self) -> Responder {
        let ss = State::new(self.vault_responder.clone());
        Responder::new(ss)
    }
}
