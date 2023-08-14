use crate::common::crazy_vault::{CrazySigningVault, CrazyVerifyingVault};

use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_identity::models::ChangeHistory;
use ockam_identity::{Identifier, Identities, Identity, Vault};
use rand::{thread_rng, Rng};

mod common;

#[tokio::test]
async fn test_valid_identity() -> Result<()> {
    let identities = Identities::builder().build();
    let identities_creation = identities.identities_creation();
    let identity = identities_creation.create_identity().await?;

    let j: i32 = thread_rng().gen_range(1..10);
    for _ in 0..j {
        // We internally check the validity during the rotation
        identities_creation
            .rotate_identity(identity.identifier())
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_invalid_signature() -> Result<()> {
    let mut vault = Vault::create();
    let crazy_signing_vault = Arc::new(CrazySigningVault::new(0.1, vault.identity_vault));
    vault.identity_vault = crazy_signing_vault.clone();
    vault.verifying_vault = Arc::new(CrazyVerifyingVault {
        verifying_vault: vault.verifying_vault,
    });
    let identities = Identities::builder().with_vault(vault).build();
    let identities_creation = identities.identities_creation();
    let identity = identities_creation.create_identity().await?;
    let identifier = identity.identifier().clone();
    let res = check_identity(&identity).await;

    if crazy_signing_vault.forged_operation_occurred() {
        assert!(res.is_err());
        return Ok(());
    } else {
        assert!(res.is_ok())
    }

    loop {
        identities_creation.rotate_identity(&identifier).await?;

        let identity = identities.get_identity(&identifier).await?;

        let res = check_identity(&identity).await;

        if crazy_signing_vault.forged_operation_occurred() {
            assert!(res.is_err());
            break;
        } else {
            assert!(res.is_ok())
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_eject_signatures() -> Result<()> {
    let identities = Identities::builder().build();
    let identities_creation = identities.identities_creation();
    let identity = identities_creation.create_identity().await?;
    let identifier = identity.identifier().clone();

    let j: i32 = thread_rng().gen_range(1..10);
    for _ in 0..j {
        identities_creation
            .rotate_identity(identity.identifier())
            .await?;
    }

    let identity = identities
        .repository()
        .get_identity(identity.identifier())
        .await?;
    let change_history = eject_random_signature(&identity)?;
    let res = check_change_history(Some(&identifier), change_history).await;
    assert!(res.is_err());

    Ok(())
}

// TODO TEST: Test that if previous_hash value doesn't match - verification fails
// TODO TEST: Test that if previous_hash value is empty - verification fails
// TODO TEST: Test that if the new key was created earlier that the previous - verification fails

/// This function simulates an identity import to check its history
async fn check_identity(identity: &Identity) -> Result<Identity> {
    Identity::import(
        Some(identity.identifier()),
        &identity.export()?,
        Vault::create_verifying_vault(),
    )
    .await
}

async fn check_change_history(
    expected_identifier: Option<&Identifier>,
    change_history: ChangeHistory,
) -> Result<Identity> {
    Identity::import_from_change_history(
        expected_identifier,
        change_history,
        Vault::create_verifying_vault(),
    )
    .await
}

pub fn eject_random_signature(change_history: &ChangeHistory) -> Result<ChangeHistory> {
    let mut history = change_history.clone();

    let i = thread_rng().gen_range(1..history.0.len());
    let change = &mut history.0[i];
    change.previous_signature = None;

    Ok(history)
}
