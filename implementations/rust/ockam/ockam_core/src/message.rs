use crate::{
    lib::{
        fmt::{self, Debug, Display, Formatter},
        Deref, DerefMut, Vec,
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
        Ok(serde_bare::to_vec(self)?)
    }

    /// Decode an [`Encoded`] type into the Message's type.
    #[allow(clippy::ptr_arg)]
    fn decode(e: &Encoded) -> Result<Self> {
        Ok(serde_bare::from_slice(e.as_slice())?)
    }
}

// Auto-implement message trait for types that _can_ be messages
impl<T> Message for T where T: Serialize + DeserializeOwned + Send + 'static {}

// TODO: see comment in Cargo.toml about this dependency
impl From<serde_bare::Error> for crate::Error {
    fn from(_: serde_bare::Error) -> Self {
        Self::new(1, "serde_bare")
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
    return_: Route,
    onward: Route,
}

impl<M: Message> Routed<M> {
    /// Create a new Routed message wrapper
    pub fn new(inner: M, return_: Route, onward: Route) -> Self {
        Self {
            inner,
            return_,
            onward,
        }
    }

    /// Return a copy of the full return route of the wrapped message
    #[inline]
    pub fn reply(&self) -> Route {
        self.return_.clone()
    }

    /// Return a copy of the onward route for this message
    #[inline]
    pub fn onward(&self) -> Route {
        self.onward.clone()
    }

    /// Get a copy of the message sender address
    #[inline]
    pub fn sender(&self) -> Address {
        self.return_.recipient()
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
