use crate::{Profile, ProfileVault};
use ockam_core::{Address, Result};
use ockam_node::Context;

/// Builder for `Entity`
pub struct EntityBuilder<V: ProfileVault> {
    ctx: Context,
    vault: V,
}

impl<V: ProfileVault> EntityBuilder<V> {
    pub async fn new(ctx: &Context, vault: &V) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.async_try_clone().await?,
        })
    }

    // TODO: enable_credentials_signing_key

    pub async fn build(self) -> Result<Profile<V>> {
        Profile::create(&self.ctx, &self.vault).await
    }
}

#[cfg(test)]
mod test {
    use crate::EntityBuilder;
    use ockam_vault_sync_core::Vault;

    #[test]
    fn test_builder() {
        let (mut ctx, mut ex) = ockam_node::start_node();
        ex.execute(async move {
            let vault = Vault::create();
            let builder = EntityBuilder::new(&ctx, &vault).await.unwrap();
            let _ = builder.build().await.unwrap();

            ctx.stop().await.unwrap();
        })
        .unwrap();
    }
}
