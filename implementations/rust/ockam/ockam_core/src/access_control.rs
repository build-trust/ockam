use crate::compat::boxed::Box;
use crate::{LocalMessage, Result};

/// Defines the interface for message flow authorization.
///
/// # Examples
///
/// ```
/// # use ockam_core::{Result, async_trait};
/// # use ockam_core::{AccessControl, LocalMessage};
/// pub struct IdentityIdAccessControl;
///
/// #[async_trait]
/// impl AccessControl for IdentityIdAccessControl {
///     async fn is_authorized(&mut self, local_msg: &LocalMessage) -> Result<bool> {
///         // ...
///         // some authorization logic that returns one of:
///         //   ockam_core::allow()
///         //   ockam_core::deny()
///         // ...
/// #       ockam_core::deny()
///     }
/// }
/// ```
///
#[async_trait]
pub trait AccessControl: Send + Sync + 'static {
    /// Return true if the message is allowed to pass, and false if not.
    async fn is_authorized(&mut self, local_msg: &LocalMessage) -> Result<bool>;
}

/// An Access Control type that allows all messages to pass through.
pub struct AllowAll;

#[async_trait]
impl AccessControl for AllowAll {
    async fn is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        crate::allow()
    }
}

/// An Access Control type that blocks all messages from passing through.
pub struct DenyAll;

#[async_trait]
impl AccessControl for DenyAll {
    async fn is_authorized(&mut self, _local_msg: &LocalMessage) -> Result<bool> {
        crate::deny()
    }
}
