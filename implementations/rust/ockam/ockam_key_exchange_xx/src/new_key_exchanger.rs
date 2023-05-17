use crate::state::State;
use crate::{Initiator, Responder, XXVault};
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, compat::boxed::Box, Result};

use ockam_core::vault::KeyId;
use ockam_core::NewKeyExchanger;

/// Represents an XX NewKeyExchanger
pub struct XXNewKeyExchanger {
    vault: Arc<dyn XXVault>,
}

impl XXNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(vault: Arc<dyn XXVault>) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl NewKeyExchanger for XXNewKeyExchanger {
    type Initiator = Initiator;
    type Responder = Responder;

    /// Create a new initiator using the provided backing vault
    async fn initiator(&self, key_id: Option<KeyId>) -> Result<Initiator> {
        let ss = State::new(self.vault.clone(), key_id).await?;
        Ok(Initiator::new(ss))
    }

    /// Create a new responder using the provided backing vault
    async fn responder(&self, key_id: Option<KeyId>) -> Result<Responder> {
        let ss = State::new(self.vault.clone(), key_id).await?;
        Ok(Responder::new(ss))
    }
}
