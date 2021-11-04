use crate::{Entity, EntityWorker};
use ockam_core::{Address, Handle, NodeContext, Result};

/// Builder for `Entity`
pub struct EntityBuilder<C> {
    ctx: C,
    vault: Address,
}

impl<C: NodeContext> EntityBuilder<C> {
    pub async fn new(ctx: &C, vault: &Address) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.clone(),
        })
    }

    // TODO: enable_credentials_signing_key

    pub async fn build(self) -> Result<Entity<C>> {
        let address = Address::random(0);
        self.ctx
            .start_worker(address.clone().into(), EntityWorker::<C>::default())
            .await?;

        let mut entity = Entity::new(Handle::new(self.ctx, address), None);

        let _ = entity.create_profile(&self.vault).await?;

        Ok(entity)
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
            let vault = Vault::create(&ctx).await.unwrap();
            let builder = EntityBuilder::new(&ctx, &vault).await.unwrap();
            let _ = builder.build().await.unwrap();

            ctx.stop().await.unwrap();
        })
        .unwrap();
    }
}
