use crate::{Entity, EntityWorker, Handle};
use ockam_core::{Address, Result};
use ockam_node::{block_future, Context};

/// Builder for `Entity`
pub struct EntityBuilder {
    ctx: Context,
    vault: Address,
}

impl EntityBuilder {
    pub fn new(ctx: &Context, vault: &Address) -> Result<Self> {
        let child_ctx = block_future(&ctx.runtime(), async {
            ctx.new_context(Address::random(0)).await
        })?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.clone(),
        })
    }

    pub async fn async_new(ctx: &Context, vault: &Address) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;

        Ok(Self {
            ctx: child_ctx,
            vault: vault.clone(),
        })
    }

    // TODO: enable_credentials_signing_key

    pub fn build(self) -> Result<Entity> {
        block_future(&self.ctx.runtime(), async move {
            let address = Address::random(0);
            self.ctx
                .start_worker(&address, EntityWorker::default())
                .await?;

            let mut entity = Entity::new(Handle::new(self.ctx, address), None);

            let _ = entity.create_profile(&self.vault)?;

            Ok(entity)
        })
    }

    pub async fn async_build(self) -> Result<Entity> {
        let address = Address::random(0);
        self.ctx
            .start_worker(&address, EntityWorker::default())
            .await?;

        let mut entity = Entity::new(Handle::new(self.ctx, address), None);

        let _ = entity.async_create_profile(&self.vault).await?;

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
            let vault = Vault::create(&ctx).unwrap();
            let builder = EntityBuilder::new(&ctx, &vault).unwrap();
            let _ = builder.build().unwrap();

            ctx.stop().await.unwrap();
        })
        .unwrap();
    }
}
