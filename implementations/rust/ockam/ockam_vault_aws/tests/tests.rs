use ockam_core::Result;
use ockam_vault::{SecretAttributes, SigningVault, Vault};
use ockam_vault_aws::AwsSigningVault;

/// These tests need to be executed with the following environment variables
/// AWS_REGION
/// AWS_ACCESS_KEY_ID
/// AWS_SECRET_ACCESS_KEY
/// or credentials in ~/.aws/credentials

#[tokio::test]
#[ignore]
async fn test_sign_verify() -> Result<()> {
    let signing_vault = AwsSigningVault::create().await?;
    let key_id = signing_vault
        .generate_key(SecretAttributes::NistP256)
        .await?;
    let message = b"hello world";
    let signature = signing_vault.sign(&key_id, message.as_slice()).await?;
    let public_key = signing_vault.get_public_key(&key_id).await?;

    let verifier = Vault::create_verifying_vault();
    assert!(verifier.verify(&public_key, message, &signature).await?);

    signing_vault.delete_key(key_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_keys_management() -> Result<()> {
    let signing_vault = AwsSigningVault::create().await?;

    let number_of_keys1 = signing_vault.number_of_keys().await?;

    let key_id = signing_vault
        .generate_key(SecretAttributes::NistP256)
        .await?;

    let number_of_keys2 = signing_vault.number_of_keys().await?;
    assert_eq!(number_of_keys1 + 1, number_of_keys2);

    let public_key = signing_vault.get_public_key(&key_id).await?;

    let key_id2 = signing_vault.get_key_id(&public_key).await?;
    assert_eq!(key_id, key_id2);

    signing_vault.delete_key(key_id).await?;
    let number_of_keys3 = signing_vault.number_of_keys().await?;
    assert_eq!(number_of_keys2, number_of_keys3 + 1);

    Ok(())
}
