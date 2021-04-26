use crate::{Initiator, Responder, X3dhVault};
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
pub struct X3dhNewKeyExchanger<V: X3dhVault> {
    vault: V,
}

impl<V: X3dhVault> std::fmt::Debug for X3dhNewKeyExchanger<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "X3dhNewKeyExchanger {{ vault }}")
    }
}

impl<V: X3dhVault> X3dhNewKeyExchanger<V> {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: V) -> Self {
        Self { vault }
    }
}

impl<V: X3dhVault> NewKeyExchanger for X3dhNewKeyExchanger<V> {
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;

    fn initiator(&self) -> ockam_core::Result<Initiator<V>> {
        Ok(Initiator::new(self.vault.clone(), None))
    }

    fn responder(&self) -> ockam_core::Result<Responder<V>> {
        Ok(Responder::new(self.vault.clone(), None))
    }
}
