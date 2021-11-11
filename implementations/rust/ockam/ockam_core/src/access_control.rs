use crate::compat::boxed::Box;
use crate::{LocalMessage, Result};

/// Access control
#[async_trait]
pub trait AccessControl: Send + Sync + 'static {
    /// Returns true if message is allowed to pass, and false if not
    async fn msg_is_authorized(&mut self, local_msg: &LocalMessage) -> Result<bool>;
}

/// Access Control that allows any message to pass through
pub struct Passthrough;

#[async_trait]
impl AccessControl for Passthrough {
    async fn msg_is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        Ok(true)
    }
}

/// Access Control that doesn't allow all messages to pass through
pub struct NoAccess;

#[async_trait]
impl AccessControl for NoAccess {
    async fn msg_is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        Ok(false)
    }
}
