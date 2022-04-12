use crate::{Initiator, Responder, X3dhVault};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};
use ockam_key_exchange_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
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
impl<V: X3dhVault> NewKeyExchanger for X3dhNewKeyExchanger<V> {
    type Initiator = Initiator<V>;
    type Responder = Responder<V>;

    async fn initiator(&self) -> Result<Initiator<V>> {
        Ok(Initiator::new(self.vault.async_try_clone().await?, None))
    }

    async fn responder(&self) -> Result<Responder<V>> {
        Ok(Responder::new(self.vault.async_try_clone().await?, None))
    }
}
