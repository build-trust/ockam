use crate::compat::boxed::Box;
use crate::{RelayMessage, Result};
use core::fmt::Debug;

/// Defines the interface for incoming message flow authorization.
///
/// # Examples
///
/// ```
/// # use ockam_core::{Result, async_trait};
/// # use ockam_core::{IncomingAccessControl, RelayMessage};
/// #[derive(Debug)]
/// pub struct IdentityIdAccessControl;
///
/// #[async_trait]
/// impl IncomingAccessControl for IdentityIdAccessControl {
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
pub trait IncomingAccessControl: Debug + Send + Sync + 'static {
    /// Return true if the message is allowed to pass, and false if not.
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool>;
}

/// Defines the interface for outgoing message flow authorization.
///
/// # Examples
///
/// ```
/// # use ockam_core::{Result, async_trait};
/// # use ockam_core::{OutgoingAccessControl, RelayMessage};
/// #[derive(Debug)]
/// pub struct LocalAccessControl;
///
/// #[async_trait]
/// impl OutgoingAccessControl for LocalAccessControl {
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
pub trait OutgoingAccessControl: Debug + Send + Sync + 'static {
    /// Return true if the message is allowed to pass, and false if not.
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool>;
}

mod all;
mod allow_all;
mod any;
mod deny_all;
mod local;
mod onward;
mod source;

pub use all::*;
pub use allow_all::*;
pub use any::*;
pub use deny_all::*;
pub use local::*;
pub use onward::*;
pub use source::*;
