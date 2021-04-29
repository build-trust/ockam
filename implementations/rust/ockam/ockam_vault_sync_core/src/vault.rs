use crate::{VaultTrait, VaultWorker};
use ockam_core::{Address, Result};
use ockam_node::{block_future, Context};

/// Vault allows to start Vault Worker.
pub struct Vault {}

impl Vault {
    /// Start a Vault with SoftwareVault implementation.
    #[cfg(feature = "software_vault")]
    pub fn create(ctx: &Context) -> Result<Address> {
        use ockam_vault::SoftwareVault;
        Self::create_with_inner(ctx, SoftwareVault::default())
    }
    /// Start a Vault Worker with given implementation.
    pub fn create_with_inner<V: VaultTrait>(ctx: &Context, inner: V) -> Result<Address> {
        let rt = ctx.runtime();
        block_future(&rt, async {
            VaultWorker::create_with_inner(ctx, inner).await
        })
    }
}
