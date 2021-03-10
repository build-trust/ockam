use crate::{
    lib::{
        fmt::{self, Debug, Display, Formatter},
        Deref, Vec,
    },
    Address, Result, Route,
};
use serde::{de::DeserializeOwned, Serialize};

/// Alias of the type used for encoded data.
pub type Encoded = Vec<u8>;

/// A user defined message that can be serialised and deserialised
pub trait Message: Serialize + DeserializeOwned + Send + 'static {
    /// Encode the type representation into an [`Encoded`] type.
    fn encode(&self) -> Result<Encoded> {
        Ok(bincode::serialize(self)?)
    }

    /// Decode an [`Encoded`] type into the Message's type.
    #[allow(clippy::ptr_arg)]
    fn decode(e: &Encoded) -> Result<Self> {
        Ok(bincode::deserialize(e)?)
    }
}

// Auto-implement message trait for types that _can_ be messages
impl<T> Message for T where T: Serialize + DeserializeOwned + Send + 'static {}

// TODO: see comment in Cargo.toml about this dependency
impl From<bincode::Error> for crate::Error {
    fn from(_: bincode::Error) -> Self {
        Self::new(1, "bincode")
    }
}

/// A message wrapper that stores message route information
///
/// Workers can accept arbitrary message types, which may not contain
/// information about their routes.  However, the ockam worker &
/// messaging system already keeps track of this information
/// internally.  This type exposes this information to the user,
/// without requiring changes in the user message types.
pub struct Routed<M: Message> {
    inner: M,
    route: Route,
}

impl<M: Message> Routed<M> {
    /// Create a new Routed message wrapper
    pub fn new(inner: M, route: Route) -> Self {
        Self { inner, route }
    }

    /// Return a copy of the full return route of the wrapped message
    #[inline]
    pub fn reply(&self) -> Route {
        self.route.clone()
    }

    /// Get a copy of the message sender address
    #[inline]
    pub fn sender(&self) -> Address {
        self.route.recipient()
    }

    /// Consume the message wrapper
    #[inline]
    pub fn take(self) -> M {
        self.inner
    }
}

impl<M: Message> Deref for Routed<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
