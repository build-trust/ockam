use crate::compat::boxed::Box;
use crate::{LocalMessage, Result};

/// Defines the interface for message flow authorization.
#[async_trait]
pub trait AccessControl: Send + Sync + 'static {
    /// Return true if the message is allowed to pass, and false if not.
    async fn msg_is_authorized(&mut self, local_msg: &LocalMessage) -> Result<bool>;
}

/// An Access Control type that allows all messages to pass through.
pub struct Passthrough;

#[async_trait]
impl AccessControl for Passthrough {
    async fn msg_is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        Ok(true)
    }
}

/// An Access Control type that blocks all messages from passing through.
pub struct NoAccess;

#[async_trait]
impl AccessControl for NoAccess {
    async fn msg_is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        Ok(false)
    }
}
