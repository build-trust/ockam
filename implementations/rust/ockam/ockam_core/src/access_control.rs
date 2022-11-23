use crate::compat::boxed::Box;
use crate::{RelayMessage, Result, LOCAL};
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

use crate::Address;

// TODO @ac Test AllowDestinationAddress & AllowSourceAddress

/// An Access Control type that allows messages from the given source address to go through
#[derive(Debug)]
// FIXME: @ac rename
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
// FIXME: @ac rename
pub struct AllowDestinationAddress(pub Address);

#[async_trait]
impl AccessControl for AllowDestinationAddress {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = &relay_msg.onward;

        // Check if next hop is equal to expected value. Further hops are not checked
        if onward_route.next()? != &self.0 {
            return crate::deny();
        }

        crate::allow()
    }
}

/// Allows only messages to local workers
#[derive(Debug)]
pub struct LocalDestinationOnly;

#[async_trait]
impl AccessControl for LocalDestinationOnly {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = &relay_msg.onward;
        let next_hop = onward_route.next()?;

        // Check if next hop is local (note that further hops may be non-local)
        if next_hop.transport_type() != LOCAL {
            return crate::deny();
        }

        crate::allow()
    }
}

/// TODO A temporary Access Control type to help me figure things out
#[derive(Debug)]
pub struct ToDoAccessControl;

#[async_trait]
impl AccessControl for ToDoAccessControl {
    async fn is_authorized(&self, _relay_msg: &RelayMessage) -> Result<bool> {
        crate::allow()
    }
}
