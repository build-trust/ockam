use crate::{VaultTrait, VaultWorker};
use ockam_core::{Address, NodeContext, Result};

/// Vault allows to start Vault Worker.
pub struct Vault {}

impl Vault {
    /// Start a Vault with SoftwareVault implementation.
    #[cfg(feature = "software_vault")]
    pub async fn create(ctx: &impl NodeContext) -> Result<Address> {
        use ockam_vault::SoftwareVault;
        Self::create_with_inner(ctx, SoftwareVault::default()).await
    }
    /// Start a Vault Worker with given implementation.
    pub async fn create_with_inner<V: VaultTrait>(
        ctx: &impl NodeContext,
        inner: V,
    ) -> Result<Address> {
        VaultWorker::create_with_inner(ctx, inner).await
    }
}
