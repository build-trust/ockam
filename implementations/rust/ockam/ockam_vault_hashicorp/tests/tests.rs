use ockam_core::Result;
use ockam_vault::{
    SigningKeyType, SoftwareVaultForVerifyingSignatures, VaultForSigning,
    VaultForVerifyingSignatures,
};
use ockam_vault_hashicorp::HashicorpSigningVault;

#[tokio::test]
async fn test_sign_verify() -> Result<()> {
    let signing_vault = HashicorpSigningVault::create().await?;
    let handle = signing_vault
        .generate_signing_secret_key(SigningKeyType::EdDSACurve25519)
        .await?;
    let message = b"hello world";
    let signature = signing_vault.sign(&handle, message.as_slice()).await?;
    let public_key = signing_vault.get_verifying_public_key(&handle).await?;

    let verifier = SoftwareVaultForVerifyingSignatures::new();
    assert!(
        verifier
            .verify_signature(&public_key, message, &signature)
            .await?
    );

    signing_vault.delete_signing_secret_key(handle).await?;

    Ok(())
}

// #[tokio::test]
// #[ignore]
// async fn test_keys_management() -> Result<()> {
//     let signing_vault = HashicorpSigningVault::create().await?;
//
//     let number_of_keys1 = signing_vault.number_of_keys().await?;
//
//     let handle = signing_vault
//         .generate_signing_secret_key(SigningKeyType::ECDSASHA256CurveP256)
//         .await?;
//
//     let number_of_keys2 = signing_vault.number_of_keys().await?;
//     assert_eq!(number_of_keys1 + 1, number_of_keys2);
//
//     let public_key = signing_vault.get_verifying_public_key(&handle).await?;
//
//     let handle2 = signing_vault.get_secret_key_handle(&public_key).await?;
//     assert_eq!(handle, handle2);
//
//     signing_vault.delete_signing_secret_key(handle).await?;
//     let number_of_keys3 = signing_vault.number_of_keys().await?;
//     assert_eq!(number_of_keys2, number_of_keys3 + 1);
//
//     Ok(())
// }
