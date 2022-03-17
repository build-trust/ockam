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
        let child_ctx = ctx.new_context(Address::random(0)).await?;

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
    use ockam_vault::Vault;

    #[test]
    fn test_builder() {
        let (mut ctx, mut ex) = ockam_node::start_node();
        ex.execute(async move {
            let vault = Vault::create();
            let builder = IdentityBuilder::new(&ctx, &vault).await.unwrap();
            let _ = builder.build().await.unwrap();

            ctx.stop().await.unwrap();
        })
        .unwrap();
    }
}
