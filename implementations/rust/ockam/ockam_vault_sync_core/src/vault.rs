use crate::{ResultMessage, VaultRequestMessage, VaultResponseMessage, VaultSyncCoreError};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use rand::random;
use zeroize::Zeroize;

pub struct Vault {
    ctx: Context,
    vault_worker_address: Address,
}

impl Vault {
    pub(crate) fn ctx(&self) -> &Context {
        &self.ctx
    }
    pub fn vault_worker_address(&self) -> &Address {
        &self.vault_worker_address
    }
}

impl Vault {
    pub fn start_another(&self) -> Result<Self> {
        let vault_worker_address = self.vault_worker_address.clone();
        let runtime = self.ctx().runtime();

        let clone = block_future(&runtime, async move {
            Vault::start(&self.ctx, vault_worker_address).await
        })?;

        Ok(clone)
    }
}

impl Zeroize for Vault {
    fn zeroize(&mut self) {}
}

impl Vault {
    fn new(ctx: Context, vault_worker_address: Address) -> Self {
        Self {
            ctx,
            vault_worker_address,
        }
    }

    pub async fn start(ctx: &Context, vault_worker_address: Address) -> Result<Self> {
        let address: Address = random();

        let ctx = ctx.new_context(address).await?;

        let runner = Self::new(ctx, vault_worker_address);

        Ok(runner)
    }

    pub(crate) async fn send_message(&self, m: VaultRequestMessage) -> Result<()> {
        self.ctx
            .send(Route::new().append(self.vault_worker_address.clone()), m)
            .await
    }

    pub(crate) async fn receive_message(&mut self) -> Result<VaultResponseMessage> {
        let r = self
            .ctx
            .receive::<ResultMessage<VaultResponseMessage>>()
            .await?
            .take()
            .body()
            .inner();

        r.map_err(|_| VaultSyncCoreError::WorkerError.into())
    }
}
