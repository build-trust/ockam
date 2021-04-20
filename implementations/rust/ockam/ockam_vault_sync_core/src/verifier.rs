use crate::{
    VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError, VaultSyncState,
};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{PublicKey, Verifier};

impl Verifier for VaultSync {
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        match &mut self.0 {
            VaultSyncState::Worker { state } => block_future(&state.ctx().runtime(), async move {
                state
                    .send_message(VaultRequestMessage::Verify {
                        signature: signature.clone(),
                        public_key: public_key.clone(),
                        data: data.into(),
                    })
                    .await?;

                let resp = state.receive_message().await?;

                if let VaultResponseMessage::Verify(s) = resp {
                    Ok(s)
                } else {
                    Err(VaultSyncCoreError::InvalidResponseType.into())
                }
            }),
            VaultSyncState::Mutex { mutex } => {
                mutex.lock().unwrap().verify(signature, public_key, data)
            }
        }
    }
}
