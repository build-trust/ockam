use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_core::Result;
use ockam_identity::{Identity, IdentityTrait};
use ockam_node::Context;
use ockam_vault::Vault;

#[ockam_macros::test(timeout = 1000)]
async fn add_key(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();
    let e = Identity::create(&ctx, &vault).await?;

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
