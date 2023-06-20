use crate::compat::boxed::Box;
use crate::{async_trait, LocalInfo, RelayMessage, Result};
use alloc::sync::Arc;
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
    // TODO: Consider &mut self
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
    // TODO: Consider &mut self
    /// Return true if the message is allowed to pass, and false if not.
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool>;
}

/// Interface for creating outgoing access control instances based on local information
/// of the first incoming message.
/// Useful when we don't have all the information on worker bootstrap.
#[async_trait]
pub trait OutgoingAccessControlFactory: Debug + Send + Sync + 'static {
    /// Creates a new outgoing access control given the incoming message
    async fn create(&self, local_info: &[LocalInfo]) -> Result<Arc<dyn OutgoingAccessControl>>;
}

mod all;
mod allow_all;
mod any;
mod deny_all;
mod onward;
mod source;

pub use all::*;
pub use allow_all::*;
pub use any::*;
pub use deny_all::*;
pub use onward::*;
pub use source::*;
