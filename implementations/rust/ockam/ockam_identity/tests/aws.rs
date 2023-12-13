use ockam_core::Result;
use ockam_identity::models::CredentialSchemaIdentifier;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{Identities, Vault};
use ockam_vault::{SigningKeyType, VaultForSigning};
use ockam_vault_aws::AwsSigningVault;
use std::sync::Arc;
use std::time::Duration;

/// These tests needs to be executed with the following environment variables
/// AWS_REGION
/// AWS_ACCESS_KEY_ID
/// AWS_SECRET_ACCESS_KEY
/// or credentials in ~/.aws/credentials

#[tokio::test]
#[ignore]
async fn create_identity_with_aws_pregenerated_key() -> Result<()> {
    let mut vault = Vault::create().await?;
    let aws_vault = Arc::new(AwsSigningVault::create().await?);
    vault.identity_vault = aws_vault.clone();
    let identities = Identities::builder()
        .await?
        .with_vault(vault.clone())
        .build();

    // create a secret key using the AWS KMS
    let key_id = aws_vault
        .generate_signing_secret_key(SigningKeyType::ECDSASHA256CurveP256)
        .await?;

    let identifier = identities
        .identities_creation()
        .identity_builder()
        .with_existing_key(key_id.clone())
        .build()
        .await?;
    let identity = identities.get_identity(&identifier).await?;

    identities
        .identities_creation()
        .import(Some(&identifier), &identity.export()?)
        .await?;

    aws_vault.delete_signing_secret_key(key_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn create_identity_with_aws_random_key() -> Result<()> {
    let mut vault = Vault::create().await?;
    let aws_vault = Arc::new(AwsSigningVault::create().await?);
    vault.identity_vault = aws_vault.clone();
    let identities = Identities::builder()
        .await?
        .with_vault(vault.clone())
        .build();

    let identifier = identities
        .identities_creation()
        .identity_builder()
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;
    let identity = identities.get_identity(&identifier).await?;

    identities
        .identities_creation()
        .import(Some(&identifier), &identity.export()?)
        .await?;

    let key = identities
        .identities_keys()
        .get_secret_key(&identity)
        .await?;

    aws_vault.delete_signing_secret_key(key).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn create_credential_aws_key() -> Result<()> {
    let mut vault = Vault::create().await?;
    let aws_vault = Arc::new(AwsSigningVault::create().await?);
    vault.credential_vault = aws_vault.clone();
    let identities = Identities::builder()
        .await?
        .with_vault(vault.clone())
        .build();

    let identifier = identities.identities_creation().create_identity().await?;

    let purpose_key = identities
        .purpose_keys()
        .purpose_keys_creation()
        .credential_purpose_key_builder(&identifier)
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;

    identities
        .purpose_keys()
        .purpose_keys_verification()
        .verify_purpose_key_attestation(Some(&identifier), purpose_key.attestation())
        .await?;

    let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
        .with_attribute(*b"key", *b"value")
        .build();

    let credential = identities
        .credentials()
        .credentials_creation()
        .issue_credential(
            &identifier,
            &identifier,
            attributes,
            Duration::from_secs(60 * 60),
        )
        .await?;

    identities
        .credentials()
        .credentials_verification()
        .verify_credential(Some(&identifier), &[identifier.clone()], &credential)
        .await?;

    aws_vault
        .delete_signing_secret_key(purpose_key.key().clone())
        .await?;

    Ok(())
}
