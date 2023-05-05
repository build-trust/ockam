use ockam_core::vault::{
    SecretAttributes, SecretPersistence, SecretType, SecretVault, Signer, Verifier,
};
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::Vault;

#[ockam_macros::test]
async fn full_flow(ctx: &mut Context) -> Result<()> {
    // Start service
    let vault = Vault::create();

    let key_id = vault
        .secret_generate(SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            0,
        ))
        .await?;

    let public_key = vault.secret_public_key_get(&key_id).await?;

    // Sign some data
    let signature = vault.sign(&key_id, b"test".as_slice()).await?;

    // Verify the signature
    let verified = vault
        .verify(&signature, &public_key, b"test".as_slice())
        .await?;
    assert!(verified);

    ctx.stop().await
}
