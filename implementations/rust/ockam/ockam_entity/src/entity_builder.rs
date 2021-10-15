use crate::{Entity, EntityWorker};
use ockam_core::{Address, Result};
use ockam_node::{Context, Handle};

/// Builder for `Entity`
pub struct EntityBuilder {
    ctx: Context,
    vault: Address,
}

impl EntityBuilder {
    pub async fn new(ctx: &Context, vault: &Address) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.clone(),
        })
    }

    // TODO: enable_credentials_signing_key

    pub async fn build(self) -> Result<Entity> {
        let address = Address::random(0);
        self.ctx
            .start_worker(&address, EntityWorker::default())
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
