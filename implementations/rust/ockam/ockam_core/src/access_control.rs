use crate::compat::boxed::Box;
use crate::{RelayMessage, Result};
use core::fmt::Debug;

/// Defines the interface for message flow authorization.
///
/// # Examples
///
/// ```
/// # use ockam_core::{Result, async_trait};
/// # use ockam_core::{AccessControl, RelayMessage};
/// #[derive(Debug)]
/// pub struct IdentityIdAccessControl;
///
/// #[async_trait]
/// impl AccessControl for IdentityIdAccessControl {
///     async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
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
#[allow(clippy::wrong_self_convention)]
pub trait AccessControl: Debug + Send + Sync + 'static {
    /// Return true if the message is allowed to pass, and false if not.
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool>;
}

mod all;
mod allow_all;
mod any;
mod deny_all;
mod destination;
mod local;
mod source;

pub use all::*;
pub use allow_all::*;
pub use any::*;
pub use deny_all::*;
pub use destination::*;
pub use local::*;
pub use source::*;
