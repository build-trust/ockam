use crate::{Identity, IdentityVault};
use ockam_core::{Address, Result};
use ockam_node::Context;

/// Builder for `Identity`
pub struct IdentityBuilder<V: IdentityVault> {
    ctx: Context,
    vault: V,
}

impl<V: IdentityVault> IdentityBuilder<V> {
    pub async fn new(ctx: &Context, vault: &V) -> Result<Self> {
        let child_ctx = ctx
            .new_detached(Address::random_tagged("IdentityBuilder.detached"))
            .await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.async_try_clone().await?,
        })
    }

    pub async fn build(self) -> Result<Identity<V>> {
        Identity::create(&self.ctx, &self.vault).await
    }
}

#[cfg(test)]
mod test {
    use crate::IdentityBuilder;
    use ockam_core::Result;
    use ockam_node::Context;
    use ockam_vault::Vault;

    #[ockam_macros::test]
    async fn test_builder(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();
        let builder = IdentityBuilder::new(&ctx, &vault).await.unwrap();
        let _ = builder.build().await.unwrap();

        ctx.stop().await
    }
}
