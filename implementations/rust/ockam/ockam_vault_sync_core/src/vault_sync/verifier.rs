use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
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
        block_future(&self.ctx.runtime(), async move {
            self.send_message(VaultRequestMessage::Verify {
                signature: signature.clone(),
                public_key: public_key.clone(),
                data: data.into(),
            })
            .await?;

            let resp = self.receive_message().await?;

            if let VaultResponseMessage::Verify(s) = resp {
                Ok(s)
            } else {
                Err(VaultSyncCoreError::InvalidResponseType.into())
            }
        })
    }
}
