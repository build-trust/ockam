use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_core::Result;
use ockam_entity::{Identity, Profile};
use ockam_node::Context;
use ockam_vault_sync_core::Vault;

#[ockam_macros::test(timeout = 1000)]
async fn add_key(ctx: &mut Context) -> Result<()> {
    let mut vault = Vault::create();
    let mut e = Profile::create(&ctx, &vault).await?;

    let key = vault
        .secret_generate(SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            32,
        ))
        .await?;

    e.add_key("test".into(), &key).await?;

    ctx.stop().await
}
