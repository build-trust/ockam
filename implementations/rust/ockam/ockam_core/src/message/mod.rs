use crate::{
    compat::{
        string::{String, ToString},
        vec::Vec,
    },
    Address, LocalMessage, Result, Route, TransportMessage,
};
use core::{
    fmt::{self, Debug, Display, Formatter},
    ops::{Deref, DerefMut},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Alias of the type used for encoded data.
pub type Encoded = Vec<u8>;

/// A user-defined protocol identifier.
///
/// When creating workers that should asynchronously speak different
/// protocols, this identifier can be used to switch message parsing
/// between delegated workers, each responsible for only one protocol.
///
/// TODO @deprecated supplanted by the new metadata message types in
///      `ockam::OckamMessage`
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct ProtocolId(String);

impl ProtocolId {
    /// Create a `None` protocol Id (with left pad).
    pub fn none() -> Self {
        Self(String::new())
    }

    /// Use the first 8 bytes of a string as the protocol ID.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    /// Return the protocol as a `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&'static str> for ProtocolId {
    fn from(s: &'static str) -> Self {
        Self::from_str(s)
    }
}

impl Display for ProtocolId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Encode the type into an [`Encoded`] type.
pub trait Encodable {
    /// Encode the type into an [`Encoded`] type.
    fn encode(&self) -> Result<Encoded>;
}

/// Decode a slice.
pub trait Decodable: Sized {
    /// Decode a slice.
    #[allow(clippy::ptr_arg)]
    fn decode(e: &[u8]) -> Result<Self>;
}

/// A user defined message that can be serialised and deserialized.
pub trait Message: Encodable + Decodable + Send + 'static {}

impl Message for () {}
impl Message for Vec<u8> {}
impl Message for String {}

// Auto-implement message trait for types that _can_ be messages.
impl<T> Encodable for T
where
    T: Serialize,
{
    fn encode(&self) -> Result<Encoded> {
        Ok(serde_bare::to_vec(self)?)
    }
}

// Auto-implement message trait for types that _can_ be messages.
impl<T> Decodable for T
where
    T: DeserializeOwned,
{
    fn decode(encoded: &[u8]) -> Result<Self> {
        Ok(serde_bare::from_slice(encoded)?)
    }
}

impl From<serde_bare::error::Error> for crate::Error {
    fn from(_: serde_bare::error::Error) -> Self {
        Self::new(1, "serde_bare")
    }
}

/// A message wrapper that provides message route information.
///
/// Workers can accept arbitrary message types, which may not contain
/// information about their routes.
///
/// However, the Ockam worker and messaging system already keeps track
/// of this information internally.
///
/// This type makes it possible to expose this information to the
/// user, without requiring changes to the user's message types.
///
/// # Examples
///
/// See `ockam_node::WorkerRelay` for a usage example.
///
pub struct Routed<M: Message> {
    // The wrapped message.
    inner: M,
    // The address of the wrapped message.
    msg_addr: Address,
    // A `LocalMessage` that contains routing information for the wrapped message.
    local_msg: LocalMessage,
}

impl<M: Message> Routed<M> {
    /// Create a new `Routed` message wrapper from the given message,
    /// message address and a local message that contains routing
    /// information.
    pub fn new(inner: M, msg_addr: Address, local_msg: LocalMessage) -> Self {
        Self {
            inner,
            msg_addr,
            local_msg,
        }
    }

    #[doc(hidden)]
    /// Return a copy of the wrapped message address and the local message.
    pub fn dissolve(&self) -> (Address, LocalMessage) {
        (self.msg_addr.clone(), self.local_msg.clone())
    }

    /// Return a copy of the message address.
    #[inline]
    pub fn msg_addr(&self) -> Address {
        self.msg_addr.clone()
    }

    /// Return a copy of the onward route for the wrapped message.
    #[inline]
    pub fn onward_route(&self) -> Route {
        self.local_msg.transport().onward_route.clone()
    }

    /// Return a copy of the full return route for the wrapped message.
    #[inline]
    pub fn return_route(&self) -> Route {
        self.local_msg.transport().return_route.clone()
    }
    /// Return a copy of the sender address for the wrapped message.
    #[inline]
    pub fn sender(&self) -> Address {
        self.local_msg.transport().return_route.recipient()
    }

    /// Consume the message wrapper and return the original message.
    #[inline]
    pub fn body(self) -> M {
        self.inner
    }

    /// Return a reference to the wrapped message.
    #[inline]
    pub fn as_body(&self) -> &M {
        &self.inner
    }

    /// Consume the message wrapper and return the underlying local message.
    #[inline]
    pub fn into_local_message(self) -> LocalMessage {
        self.local_msg
    }

    /// Consume the message wrapper and return the underlying transport message.
    #[inline]
    pub fn into_transport_message(self) -> TransportMessage {
        self.into_local_message().into_transport_message()
    }

    /// Return a reference to the underlying local message.
    #[inline]
    pub fn local_message(&self) -> &LocalMessage {
        &self.local_msg
    }

    /// Return a reference to the underlying transport message's binary payload.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.local_msg.transport().payload
    }

    /// Consume the message wrapper and return the underlying transport message's binary payload.
    #[inline]
    pub fn take_payload(self) -> Vec<u8> {
        self.local_msg.into_transport_message().payload
    }
}

impl<M: Message> Deref for Routed<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: Message> DerefMut for Routed<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<M: Message + PartialEq> PartialEq<M> for Routed<M> {
    fn eq(&self, o: &M) -> bool {
        &self.inner == o
    }
}

impl<M: Message + Debug> Debug for Routed<M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<M: Message + Display> Display for Routed<M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// A passthrough marker message type.
///
/// This is a special message type which will enable your worker to
/// accept _any_ typed message, by ignoring the type information in
/// the payload.
///
/// This is especially useful for implementing middleware workers
/// which need access to the route information of a message, without
/// understanding its payload.
///
/// # Examples
///
/// ```ignore
/// use ockam::{hex, Any, Context, Result, Routed, Worker};
///
/// pub struct Logger;
///
/// #[ockam::worker]
/// impl Worker for Logger {
///     type Context = Context;
///     type Message = Any;
///
///     /// This Worker will take any incoming message, print out the payload
///     /// and then forward it to the next hop in its onward route.
///     async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
///         let mut local_msg = msg.into_local_message();
///         let transport_msg = local_msg.transport_mut();
///         transport_msg.onward_route.step()?;
///         transport_msg.return_route.modify().prepend(ctx.address());
///
///         let payload = transport_msg.payload.clone();
///
///         if let Ok(str) = String::from_utf8(payload.clone()) {
///             println!("Address: {}, Received string: {}", ctx.address(), str);
///         } else {
///             println!("Address: {}, Received binary: {}", ctx.address(), hex::encode(&payload));
///         }
///
///         ctx.forward(local_msg).await
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, crate::Message)]
pub struct Any;

impl Display for Any {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Any Message")
    }
}

impl Encodable for Any {
    fn encode(&self) -> Result<Encoded> {
        Ok(vec![])
    }
}

impl Decodable for Any {
    fn decode(_: &[u8]) -> Result<Self> {
        Ok(Self)
    }
}
