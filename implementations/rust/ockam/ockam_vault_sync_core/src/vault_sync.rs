use crate::{ResultMessage, Vault, VaultRequestMessage, VaultResponseMessage, VaultTrait};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use rand::random;
use std::sync::{Arc, Mutex};
use tracing::info;
use zeroize::Zeroize;

pub(crate) struct WorkerState {
    ctx: Context,
    vault_worker_address: Address,
    error_domain: &'static str,
}

impl WorkerState {
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

impl WorkerState {
    pub(crate) fn ctx(&self) -> &Context {
        &self.ctx
    }
}

pub(crate) enum VaultSyncState {
    Worker { state: WorkerState },
    Mutex { mutex: Arc<Mutex<dyn VaultTrait>> },
}

/// Vault sync wrapper
pub struct VaultSync(pub(crate) VaultSyncState);

impl Clone for VaultSync {
    fn clone(&self) -> Self {
        self.start_another().unwrap()
    }
}

impl VaultSync {
    /// Start another Vault at the same address.
    pub fn start_another(&self) -> Result<Self> {
        match &self.0 {
            VaultSyncState::Worker { state } => {
                let vault_worker_address = state.vault_worker_address.clone();
                let runtime = state.ctx().runtime();

                let clone = block_future(&runtime, async move {
                    VaultSync::create(&state.ctx, vault_worker_address, state.error_domain).await
                })?;

                Ok(clone)
            }
            VaultSyncState::Mutex { mutex } => Ok(Self(VaultSyncState::Mutex {
                mutex: mutex.clone(),
            })),
        }
    }
}

impl Zeroize for VaultSync {
    fn zeroize(&mut self) {}
}

impl VaultSync {
    /// Create and start a new Vault using Mutex.
    pub fn create_with_mutex<T: VaultTrait>(vault: T) -> Self {
        info!("Starting Mutex Vault");

        Self(VaultSyncState::Mutex {
            mutex: Arc::new(Mutex::new(vault)),
        })
    }

    /// Create and start a new Vault using Worker.
    pub async fn create(
        ctx: &Context,
        vault_worker_address: Address,
        error_domain: &'static str,
    ) -> Result<Self> {
        let address: Address = random();

        info!("Starting Vault at {}", &address);

        let ctx = ctx.new_context(address).await?;

        let state = WorkerState {
            ctx,
            vault_worker_address,
            error_domain,
        };

        let vault = VaultSyncState::Worker { state };

        Ok(Self(vault))
    }

    /// Start a Vault.
    pub async fn start<T: VaultTrait>(ctx: &Context, vault: T) -> Result<Self> {
        let error_domain = vault.error_domain();

        let vault_address = Vault::start(ctx, vault).await?;

        Self::create(ctx, vault_address, error_domain).await
    }
}
