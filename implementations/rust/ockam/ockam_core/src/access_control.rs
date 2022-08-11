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

/// Convenience structure for passing around an incoming/outgoing
/// [`AccessControl`] pair.
pub struct AccessControlPair<IN, OUT> {
    /// Incoming `AccessControl`
    pub incoming: IN,
    /// Outgoing `AccessControl`
    pub outgoing: OUT,
}

mod all;
mod allow_all;
mod any;
mod deny_all;

pub use all::*;
pub use allow_all::*;
pub use any::*;
pub use deny_all::*;

/// A test Access Control type to check outgoing AccessControl
#[derive(Debug)]
pub struct AllowAllOutgoing;

#[async_trait]
impl AccessControl for AllowAllOutgoing {
    async fn is_authorized(&self, _relay_msg: &RelayMessage) -> Result<bool> {
        crate::allow()
    }
}

use crate::Address;

/// An Access Control type that allows messages from the given source address to go through
#[derive(Debug)]
pub struct AllowSourceAddress(pub Address);

#[async_trait]
impl AccessControl for AllowSourceAddress {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if relay_msg.source == self.0 {
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

/// An Access Control type that allows messages to the given destination address to go through
#[derive(Debug)]
pub struct AllowDestinationAddress(pub Address);

#[async_trait]
impl AccessControl for AllowDestinationAddress {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if relay_msg.destination == self.0 {
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

/// TODO A temporary Access Control type for credentials while I figure things out
#[derive(Debug)]
pub struct CredentialAccessControl {}

#[async_trait]
impl AccessControl for CredentialAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let local_info = relay_msg.local_msg.local_info();
        tracing::error!("CredentialAccessControl <= {:?}", local_info);
        tracing::error!("CredentialAccessControl <= {:?}", relay_msg);
        crate::allow()
    }
}
