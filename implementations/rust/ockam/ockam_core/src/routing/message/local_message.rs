#[cfg(feature = "std")]
use crate::OpenTelemetryContext;
use crate::{compat::vec::Vec, route, Address, Message, Route, TransportMessage};
use crate::{LocalInfo, Result};
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};

/// A message type that is routed locally within a single node.
///
/// [`LocalMessage`] contains:
///  - An onward route for the message
///  - A return route
///  - A binary payload
///  - Additional metadata as [`LocalInfo`] in binary format, that can be added by Workers
///    within the same node.
///
/// A [`LocalMessage`] can be converted from a [`TransportMessage`] that has just been deserialized
/// from some binary data arriving on a node.
///
/// It can also be converted to a [`TransportMessage`] to be serialized and sent to another node.
///
/// When a [`LocalMessage`] has been processed by a worker, its `onward_route` and `return_route` need to be modified
/// before the message is sent to another worker. This is generally done by:
///
///  - popping the first address of the onward route (which should be the worker address)
///  - push a new return address at the front of the return route (this can be the current worker address but this can
///    also be the address of another worker, depending on the desired topology).
///
/// Therefore, a certain number of functions are available on [`LocalMessage`] to manipulate the onward and return routes:
///
/// - pop_front_onward_route: remove the first address of the onward route
/// - replace_front_onward_route: replace the first address of the onward route with another address
/// - push_front_onward_route: add an address at the front of the onward route
/// - prepend_front_onward_route: prepend a whole route at the front of the onward route
/// - set_onward_route: set a new route for the onward route
///
/// There are similar functions for return routes. All modification functions can be composed. For example:
///
/// self.pop_front_onward_route()?.prepend_front_return_route(&new_route)
///
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct LocalMessage {
    /// Onward message route.
    onward_route: Route,
    /// Return message route. This field must be populated by routers handling this message along the way.
    return_route: Route,
    /// The message payload.
    payload: Vec<u8>,
    /// Local information added by workers to give additional context to the message
    /// independently from its payload. For example this can be used to store the identifier that
    /// was used to encrypt the payload
    local_info: Vec<LocalInfo>,
    /// Local tracing context
    #[cfg(feature = "std")]
    tracing_context: OpenTelemetryContext,
}

impl LocalMessage {
    /// Return the message onward route
    pub fn onward_route(&self) -> Route {
        self.onward_route.clone()
    }

    /// Return a reference to the message onward route
    pub fn onward_route_ref(&self) -> &Route {
        &self.onward_route
    }

    /// Return the next address on the onward route
    pub fn next_on_onward_route(&self) -> Result<Address> {
        Ok(self.onward_route.next()?.clone())
    }

    /// Return true if an address exists on the onward route
    pub fn has_next_on_onward_route(&self) -> bool {
        self.onward_route.next().is_ok()
    }

    /// Remove the first address of the onward route
    pub fn pop_front_onward_route(mut self) -> Result<Self> {
        let _ = self.onward_route.step()?;
        Ok(self)
    }

    /// Prepend an address on the onward route
    pub fn push_front_onward_route(mut self, address: &Address) -> Self {
        self.onward_route.modify().prepend(address.clone());
        self
    }

    /// Replace the first address on the onward route
    pub fn replace_front_onward_route(self, address: &Address) -> Result<Self> {
        Ok(self
            .pop_front_onward_route()?
            .push_front_onward_route(address))
    }

    /// Prepend a route to the onward route
    pub fn prepend_front_onward_route(mut self, route: &Route) -> Self {
        self.onward_route.modify().prepend_route(route.clone());
        self
    }

    /// Set the message onward route
    pub fn set_onward_route(mut self, route: Route) -> Self {
        self.onward_route = route;
        self
    }

    /// Return the message return route
    pub fn return_route(&self) -> Route {
        self.return_route.clone()
    }

    /// Return a reference to the message return route
    pub fn return_route_ref(&self) -> &Route {
        &self.return_route
    }

    /// Set the message return route
    pub fn set_return_route(mut self, route: Route) -> Self {
        self.return_route = route;
        self
    }

    /// Prepend an address to the return route
    pub fn push_front_return_route(mut self, address: &Address) -> Self {
        self.return_route.modify().prepend(address.clone());
        self
    }

    /// Prepend a route to the return route
    pub fn prepend_front_return_route(mut self, route: &Route) -> Self {
        self.return_route.modify().prepend_route(route.clone());
        self
    }

    /// Remove the first address on the onward route and push another address on the return route
    pub fn step_forward(self, address: &Address) -> Result<Self> {
        Ok(self
            .pop_front_onward_route()?
            .push_front_return_route(address))
    }

    /// Return the message payload
    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    /// Return a reference to the message payload
    pub fn payload_ref(&self) -> &[u8] {
        &self.payload
    }

    /// Return a mutable reference to the message payload
    pub fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut self.payload
    }

    /// Prepend an address to the return route
    pub fn set_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    /// Return the message local info
    pub fn local_info(&self) -> Vec<LocalInfo> {
        self.local_info.clone()
    }

    /// Return a reference to the message local info
    pub fn local_info_ref(&self) -> &[LocalInfo] {
        &self.local_info
    }

    /// Return a mutable reference to the message local info
    pub fn local_info_mut(&mut self) -> &mut Vec<LocalInfo> {
        &mut self.local_info
    }

    /// Clear all [`LocalInfo`] entries
    pub fn clear_local_info(&mut self) {
        self.local_info.clear()
    }

    /// Get the tracing context associated to this local message
    #[cfg(feature = "std")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        self.tracing_context.clone()
    }

    /// Create a [`LocalMessage`] from a decoded [`TransportMessage`]
    pub fn from_transport_message(transport_message: TransportMessage) -> LocalMessage {
        cfg_if! {
            if #[cfg(feature = "std")] {
                LocalMessage::new()
                    .with_tracing_context(transport_message.tracing_context())
                    .with_onward_route(transport_message.onward_route)
                    .with_return_route(transport_message.return_route)
                    .with_payload(transport_message.payload)
            } else {
                LocalMessage::new()
                    .with_onward_route(transport_message.onward_route)
                    .with_return_route(transport_message.return_route)
                    .with_payload(transport_message.payload)
            }
        }
    }

    /// Create a [`TransportMessage`] from a [`LocalMessage`]
    pub fn into_transport_message(self) -> TransportMessage {
        let transport_message =
            TransportMessage::v1(self.onward_route(), self.return_route(), self.payload());
        cfg_if! {
            if #[cfg(feature = "std")] {
                // make sure to pass the latest tracing context
                transport_message.set_tracing_context(self.tracing_context.update())
            } else {
                transport_message
            }
        }
    }
}

impl Default for LocalMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalMessage {
    /// Create a new `LocalMessage` from the provided transport message and local information.
    fn make(
        onward_route: Route,
        return_route: Route,
        payload: Vec<u8>,
        local_info: Vec<LocalInfo>,
    ) -> Self {
        LocalMessage {
            onward_route,
            return_route,
            payload,
            local_info,
            #[cfg(feature = "std")]
            tracing_context: OpenTelemetryContext::current(),
        }
    }

    /// Create a `LocalMessage` with default values, in order to build it with
    /// the withXXX methods
    pub fn new() -> Self {
        LocalMessage::make(route![], route![], vec![], vec![])
    }

    /// Specify the onward route for the message
    pub fn with_onward_route(self, onward_route: Route) -> Self {
        Self {
            onward_route,
            ..self
        }
    }

    /// Specify the return route for the message
    pub fn with_return_route(self, return_route: Route) -> Self {
        Self {
            return_route,
            ..self
        }
    }

    /// Specify the payload for the message
    pub fn with_payload(self, payload: Vec<u8>) -> Self {
        Self { payload, ..self }
    }

    /// Specify the local information for the message
    pub fn with_local_info(self, local_info: Vec<LocalInfo>) -> Self {
        Self { local_info, ..self }
    }

    /// Specify the tracing context
    #[cfg(feature = "std")]
    pub fn with_tracing_context(self, tracing_context: OpenTelemetryContext) -> Self {
        Self {
            tracing_context,
            ..self
        }
    }
}
