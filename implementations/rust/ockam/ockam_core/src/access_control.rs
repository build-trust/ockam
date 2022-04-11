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
///     async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
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
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool>;
}

mod all;
mod allow_all;
mod any;
mod deny_all;

pub use all::*;
pub use allow_all::*;
pub use any::*;
pub use deny_all::*;
