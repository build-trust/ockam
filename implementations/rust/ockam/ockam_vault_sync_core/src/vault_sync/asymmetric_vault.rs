use crate::{VaultRequestMessage, VaultResponseMessage, VaultSync, VaultSyncCoreError};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{NodeContext, Result};
use ockam_vault_core::{AsymmetricVault, PublicKey, Secret};

#[async_trait]
impl<C: NodeContext> AsymmetricVault for VaultSync<C> {
    async fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        let resp = self
            .call(VaultRequestMessage::EcDiffieHellman {
                context: context.clone(),
                peer_public_key: peer_public_key.clone(),
            })
            .await?;

        if let VaultResponseMessage::EcDiffieHellman(s) = resp {
            Ok(s)
        } else {
            Err(VaultSyncCoreError::InvalidResponseType.into())
        }
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
    fn ec_diffie_hellman_curve25519() {}
}
