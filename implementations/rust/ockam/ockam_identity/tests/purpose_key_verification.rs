use crate::common::crazy_vault::{CrazySigningVault, CrazyVerifyingVault};

use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_identity::{identities, Identities, Vault};

mod common;

#[tokio::test]
async fn test_invalid_signature() -> Result<()> {
    let mut vault = Vault::create();
    let crazy_signing_vault = Arc::new(CrazySigningVault::new(0.1, vault.identity_vault));
    vault.identity_vault = crazy_signing_vault.clone();
    vault.verifying_vault = Arc::new(CrazyVerifyingVault {
        verifying_vault: vault.verifying_vault,
    });

    let identities_remote = identities();
    let identities = Identities::builder().with_vault(vault).build();
    let identities_creation = identities.identities_creation();
    let identity = identities_creation.create_identity().await?;

    if crazy_signing_vault.forged_operation_occurred() {
        return Ok(());
    }

    let purpose_keys = identities.purpose_keys();

    identities_remote
        .identities_creation()
        .update_identity(&identity)
        .await?;

    loop {
        let purpose_key = purpose_keys
            .purpose_keys_creation()
            .create_credential_purpose_key(identity.identifier())
            .await?;

        let res = identities_remote
            .purpose_keys()
            .purpose_keys_verification()
            .verify_purpose_key_attestation(Some(identity.identifier()), purpose_key.attestation())
            .await;

        if crazy_signing_vault.forged_operation_occurred() {
            assert!(res.is_err());
            break;
        } else {
            assert!(res.is_ok())
        }
    }

    Ok(())
}
