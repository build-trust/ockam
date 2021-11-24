use ockam_core::Result;
use ockam_entity::{Entity, Identity};
use ockam_node::Context;
use ockam_vault::SoftwareVault;
use ockam_vault_core::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_vault_sync_core::VaultSync;

#[ockam_macros::test(timeout = 1000)]
async fn add_key(ctx: &mut Context) -> Result<()> {
    let mut vault = VaultSync::create(&ctx, SoftwareVault::default()).await?;
    let mut e = Entity::create(&ctx, &vault.address()).await?;

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
