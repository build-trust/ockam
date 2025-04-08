use crate::Context;
use ockam_core::async_trait;

/// This trait can be used to integrate transports into a node
#[async_trait]
pub trait HasContext {
    /// Return a cloned context
    fn get_context(&self) -> &Context;
}
