use crate::{Identity, IdentityVault};
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, DenyAll, Result};
use ockam_node::Context;

/// Builder for `Identity`
pub struct IdentityBuilder {
    ctx: Context,
    vault: Arc<dyn IdentityVault>,
}

impl IdentityBuilder {
    /// Constructor
    pub async fn new(ctx: &Context, vault: Arc<dyn IdentityVault>) -> Result<Self> {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("IdentityBuilder.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.clone(),
        })
    }

    /// Build an `Identity`
    pub async fn build(self) -> Result<Identity> {
        Identity::create_arc(&self.ctx, self.vault).await
    }
}

#[cfg(test)]
mod test {
    use crate::{IdentityBuilder, IdentityVault};
    use ockam_core::Result;
    use ockam_node::Context;
    use ockam_vault::Vault;
    use std::sync::Arc;

    #[ockam_macros::test]
    async fn test_builder(ctx: &mut Context) -> Result<()> {
        let vault: Arc<dyn IdentityVault> = Arc::new(Vault::create());
        let builder = IdentityBuilder::new(ctx, vault).await.unwrap();
        let _ = builder.build().await.unwrap();

        ctx.stop().await
    }
}
