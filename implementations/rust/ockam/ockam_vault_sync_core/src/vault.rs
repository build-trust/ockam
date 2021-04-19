use crate::{
    ResultMessage, VaultRequestMessage, VaultResponseMessage, VaultWorker, VaultWorkerTrait,
};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use rand::random;
use tracing::info;
use zeroize::Zeroize;

/// Vault worker reference.
pub struct Vault {
    ctx: Context,
    vault_worker_address: Address,
    error_domain: &'static str,
}

impl Vault {
    /// The Conttext of the worker.
    pub(crate) fn ctx(&self) -> &Context {
        &self.ctx
    }
    /// Address of the Vault worker.
    pub fn vault_worker_address(&self) -> &Address {
        &self.vault_worker_address
    }

    /// Error dmain.
    pub fn error_domain(&self) -> &'static str {
        self.error_domain
    }
}

impl Vault {
    /// Start another Vault at the same address.
    pub fn start_another(&self) -> Result<Self> {
        let vault_worker_address = self.vault_worker_address.clone();
        let runtime = self.ctx().runtime();

        let clone = block_future(&runtime, async move {
            Vault::create(&self.ctx, vault_worker_address, self.error_domain).await
        })?;

        Ok(clone)
    }
}

impl Zeroize for Vault {
    fn zeroize(&mut self) {}
}

impl Vault {
    /// Create a new Vault.
    fn new(ctx: Context, vault_worker_address: Address, error_domain: &'static str) -> Self {
        Self {
            ctx,
            vault_worker_address,
            error_domain,
        }
    }

    /// Create and start a new Vault.
    pub async fn create(
        ctx: &Context,
        vault_worker_address: Address,
        error_domain: &'static str,
    ) -> Result<Self> {
        let address: Address = random();

        info!("Starting Vault at {}", &address);

        let ctx = ctx.new_context(address).await?;

        let runner = Self::new(ctx, vault_worker_address, error_domain);

        Ok(runner)
    }

    /// Start a Vault.
    pub async fn start<T: VaultWorkerTrait>(ctx: &Context, vault: T) -> Result<Self> {
        let error_domain = T::error_domain();

        let vault_address = VaultWorker::start(ctx, vault).await?;

        Self::create(ctx, vault_address, error_domain).await
    }

    pub(crate) async fn send_message(&self, m: VaultRequestMessage) -> Result<()> {
        self.ctx
            .send(Route::new().append(self.vault_worker_address.clone()), m)
            .await
    }

    pub(crate) async fn receive_message(&mut self) -> Result<VaultResponseMessage> {
        self.ctx
            .receive::<ResultMessage<VaultResponseMessage>>()
            .await?
            .take()
            .body()
            .inner(self.error_domain)
    }
}
