use crate::{Vault, VaultRequestMessage, VaultResponseMessage, VaultTrait};
#[cfg(not(feature = "std"))]
use ockam_core::compat::rand::random;
use ockam_core::{async_trait::async_trait, Address, AsyncTryClone, Result, ResultMessage, Route};
use ockam_node::{block_future, Context, Handle};
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
    // ctx: Context,
    // vault_worker_address: Address,
    handle: Handle
}

impl VaultSync {
    // Todo: mark as deprecated
    // #[deprecated]
    #[allow(unused)]
    pub(crate) async fn send_message(&self, m: VaultRequestMessage) -> Result<()> {
        self.handle.ctx()
            .send(Route::new().append(self.handle.address().clone()), m)
            .await
    }

    // Todo: mark as deprecated
    // #[deprecated]
    #[allow(unused)]
    pub(crate) async fn receive_message(&mut self) -> Result<VaultResponseMessage> {
        self.handle.ctx_mut()
            .receive::<ResultMessage<VaultResponseMessage>>()
            .await?
            .take()
            .body()
            .into()
    }

    pub(crate) fn call(&mut self, msg: VaultRequestMessage) -> Result<VaultResponseMessage> {
        self.handle
            .call::<VaultRequestMessage, ResultMessage<VaultResponseMessage>>(
                msg
            )?.into()
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
        let vault_worker_address = self.handle.address().clone();

        let clone = VaultSync::create_with_worker(&self.handle.ctx(), &vault_worker_address)?;

        Ok(clone)
    }
}

impl Zeroize for VaultSync {
    fn zeroize(&mut self) {}
}

impl VaultSync {
    /// Create and start a new Vault using Worker.
    pub fn create_with_worker(ctx: &Context, vault: &Address) -> Result<Self> {
        let address: Address = random();

        debug!("Starting VaultSync at {}", &address);

        let ctx = block_future(
            &ctx.runtime(),
            async move { ctx.new_context(address).await },
        )?;


        Ok(Self {
            handle: Handle::new(ctx, vault.clone())
        })
    }

    /// Start a Vault.
    pub fn create<T: VaultTrait>(ctx: &Context, vault: T) -> Result<Self> {
        let vault_address = Vault::create_with_inner(ctx, vault)?;

        Self::create_with_worker(ctx, &vault_address)
    }

    /// Return the Vault worker address
    pub fn address(&self) -> Address {
        self.handle.address().clone()
    }
}
