use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::vault::{Buffer, Secret, SymmetricVault};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl SymmetricVault for VaultSync {
    async fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let resp = self
            .call(VaultRequestMessage::AeadAesGcmEncrypt {
                context: context.clone(),
                plaintext: plaintext.into(),
                nonce: nonce.into(),
                aad: aad.into(),
            })
            .await?;

        if let VaultResponseMessage::AeadAesGcmEncrypt(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }

    async fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let resp = self
            .call(VaultRequestMessage::AeadAesGcmDecrypt {
                context: context.clone(),
                cipher_text: cipher_text.into(),
                nonce: nonce.into(),
                aad: aad.into(),
            })
            .await?;

        if let VaultResponseMessage::AeadAesGcmDecrypt(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test_sync]
    fn encryption() {}
}
