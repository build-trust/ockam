use crate::{Initiator, Responder, X3dhVault};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(Clone)]
pub struct X3dhNewKeyExchanger<V: X3dhVault> {
    vault: V,
}

impl<V: X3dhVault> core::fmt::Debug for X3dhNewKeyExchanger<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "X3dhNewKeyExchanger {{ vault }}")
    }
}

impl<V: X3dhVault> X3dhNewKeyExchanger<V> {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: V) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl<V: X3dhVault + Sync> NewKeyExchanger for X3dhNewKeyExchanger<V> {
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;

    fn initiator(&self) -> ockam_core::Result<Initiator<V>> {
        Ok(Initiator::new(self.vault.clone(), None))
    }

    fn responder(&self) -> ockam_core::Result<Responder<V>> {
        Ok(Responder::new(self.vault.clone(), None))
    }

    async fn async_initiator(&self) -> ockam_core::Result<Initiator<V>> {
        Ok(Initiator::new(self.vault.clone(), None))
    }

    async fn async_responder(&self) -> ockam_core::Result<Responder<V>> {
        Ok(Responder::new(self.vault.clone(), None))
    }
}
