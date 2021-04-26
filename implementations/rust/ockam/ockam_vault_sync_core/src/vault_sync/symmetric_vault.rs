use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::Result;
use ockam_node::block_future;
use ockam_vault_core::{Buffer, Secret, SymmetricVault};

impl SymmetricVault for VaultSync {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        block_future(&self.ctx.runtime(), async move {
            self.send_message(VaultRequestMessage::AeadAesGcmEncrypt {
                context: context.clone(),
                plaintext: plaintext.into(),
                nonce: nonce.into(),
                aad: aad.into(),
            })
            .await?;

            let resp = self.receive_message().await?;

            if let VaultResponseMessage::AeadAesGcmEncrypt(s) = resp {
                Ok(s)
            } else {
                Err(VaultSyncCoreError::InvalidResponseType.into())
            }
        })
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        block_future(&self.ctx.runtime(), async move {
            self.send_message(VaultRequestMessage::AeadAesGcmDecrypt {
                context: context.clone(),
                cipher_text: cipher_text.into(),
                nonce: nonce.into(),
                aad: aad.into(),
            })
            .await?;

            let resp = self.receive_message().await?;

            if let VaultResponseMessage::AeadAesGcmDecrypt(s) = resp {
                Ok(s)
            } else {
                Err(VaultSyncCoreError::InvalidResponseType.into())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test_sync]
    fn encryption() {}
}
