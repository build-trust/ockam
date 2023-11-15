use ockam_core::Result;
use ockam_identity::models::CredentialSchemaIdentifier;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{Identities, Vault};
use ockam_vault::{SigningKeyType, VaultForSigning};
use ockam_vault_hashicorp::HashicorpSigningVault;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
// #[ignore]
async fn create_identity_with_hashicorp_pregenerated_key() -> Result<()> {
    let mut vault = Vault::create();
    let hashicorp_vault = Arc::new(HashicorpSigningVault::create().await?);
    vault.identity_vault = hashicorp_vault.clone();
    let identities = Identities::builder().with_vault(vault.clone()).build();

    let key_id = hashicorp_vault
        .generate_signing_secret_key(SigningKeyType::EdDSACurve25519)
        .await?;

    let identity = identities
        .identities_creation()
        .identity_builder()
        .with_existing_key(key_id.clone())
        .build()
        .await?;

    identities
        .identities_creation()
        .import(Some(identity.identifier()), &identity.export()?)
        .await?;

    hashicorp_vault.delete_signing_secret_key(key_id).await?;

    Ok(())
}

#[tokio::test]
// #[ignore]
async fn create_identity_with_hashicorp_random_key() -> Result<()> {
    let mut vault = Vault::create();
    let hashicorp_vault = Arc::new(HashicorpSigningVault::create().await?);
    vault.identity_vault = hashicorp_vault.clone();
    let identities = Identities::builder().with_vault(vault.clone()).build();

    let identity = identities
        .identities_creation()
        .identity_builder()
        .with_random_key(SigningKeyType::EdDSACurve25519)
        .build()
        .await?;

    identities
        .identities_creation()
        .import(Some(identity.identifier()), &identity.export()?)
        .await?;

    let key = identities
        .identities_keys()
        .get_secret_key(&identity)
        .await?;

    hashicorp_vault.delete_signing_secret_key(key).await?;

    Ok(())
}

#[tokio::test]
// #[ignore]
async fn create_credential_hashicorp_key() -> Result<()> {
    let mut vault = Vault::create();
    let hashicorp_vault = Arc::new(HashicorpSigningVault::create().await?);
    vault.credential_vault = hashicorp_vault.clone();
    let identities = Identities::builder().with_vault(vault.clone()).build();

    let identity = identities.identities_creation().create_identity().await?;

    let purpose_key = identities
        .purpose_keys()
        .purpose_keys_creation()
        .credential_purpose_key_builder(identity.identifier())
        .with_random_key(SigningKeyType::EdDSACurve25519)
        .build()
        .await?;

    identities
        .purpose_keys()
        .purpose_keys_verification()
        .verify_purpose_key_attestation(Some(identity.identifier()), purpose_key.attestation())
        .await?;

    let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
        .with_attribute(*b"key", *b"value")
        .build();

    let credential = identities
        .credentials()
        .credentials_creation()
        .issue_credential(
            identity.identifier(),
            identity.identifier(),
            attributes,
            Duration::from_secs(120),
        )
        .await?;

    identities
        .credentials()
        .credentials_verification()
        .verify_credential(
            Some(identity.identifier()),
            &[identity.identifier().clone()],
            &credential,
        )
        .await?;

    hashicorp_vault
        .delete_signing_secret_key(purpose_key.key().clone())
        .await?;

    Ok(())
}
