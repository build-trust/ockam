use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::vault::{PublicKey, Signature, Verifier};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl Verifier for VaultSync {
    async fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        let resp = self
            .call(VaultRequestMessage::Verify {
                signature: signature.clone(),
                public_key: public_key.clone(),
                data: data.into(),
            })
            .await?;

        if let VaultResponseMessage::Verify(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
    }
}
