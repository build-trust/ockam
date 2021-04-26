use crate::{ResultMessage, Vault, VaultRequestMessage, VaultResponseMessage, VaultTrait};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use rand::random;
use tracing::debug;
use zeroize::Zeroize;

mod asymmetric_vault;
mod hasher;
mod key_id_vault;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod verifier;

pub use asymmetric_vault::*;
pub use hasher::*;
pub use key_id_vault::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use verifier::*;

/// Vault sync wrapper
pub struct VaultSync {
    ctx: Context,
    vault_worker_address: Address,
    error_domain: &'static str,
}

impl VaultSync {
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

impl Clone for VaultSync {
    fn clone(&self) -> Self {
        self.start_another().unwrap()
    }
}

impl VaultSync {
    /// Start another Vault at the same address.
    pub fn start_another(&self) -> Result<Self> {
        let vault_worker_address = self.vault_worker_address.clone();

        let clone =
            VaultSync::create_with_worker(&self.ctx, &vault_worker_address, self.error_domain)?;

        Ok(clone)
    }
}

impl Zeroize for VaultSync {
    fn zeroize(&mut self) {}
}

impl VaultSync {
    /// Create and start a new Vault using Worker.
    pub fn create_with_worker(
        ctx: &Context,
        vault: &Address,
        error_domain: &'static str,
    ) -> Result<Self> {
        let address: Address = random();

        debug!("Starting VaultSync at {}", &address);

        let ctx = block_future(
            &ctx.runtime(),
            async move { ctx.new_context(address).await },
        )?;

        Ok(Self {
            ctx,
            vault_worker_address: vault.clone(),
            error_domain,
        })
    }

    /// Start a Vault.
    pub fn create<T: VaultTrait>(ctx: &Context, vault: T) -> Result<Self> {
        let error_domain = vault.error_domain();

        let vault_address = Vault::create_with_inner(ctx, vault)?;

        Self::create_with_worker(ctx, &vault_address, error_domain)
    }
}
