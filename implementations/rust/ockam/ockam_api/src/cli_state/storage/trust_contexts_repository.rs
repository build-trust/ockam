use crate::cli_state::NamedTrustContext;
use ockam_core::async_trait;
use ockam_core::Result;

/// This trait supports the storage of trust context data:
///
///  - one single trust context can be set as the default one
///
#[async_trait]
pub trait TrustContextsRepository: Send + Sync + 'static {
    /// Store trust context data associated with a specific trust context name
    async fn store_trust_context(&self, trust_context: &NamedTrustContext) -> Result<()>;

    /// Get the default named trust context
    async fn get_default_trust_context(&self) -> Result<Option<NamedTrustContext>>;

    /// Set a trust context as the default one
    async fn set_default_trust_context(&self, name: &str) -> Result<()>;

    /// Get a named trust context by name
    async fn get_trust_context(&self, name: &str) -> Result<Option<NamedTrustContext>>;

    /// Get all named trust contexts
    async fn get_trust_contexts(&self) -> Result<Vec<NamedTrustContext>>;

    /// Delete a trust context
    async fn delete_trust_context(&self, name: &str) -> Result<()>;
}
