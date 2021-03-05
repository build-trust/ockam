use crate::{lib::VecDeque, Address};
use serde::{Deserialize, Serialize};

/// A full route to a peer
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Route {
    inner: VecDeque<Address>,
}

impl Route {
    /// Create an empty RouteBuilder
    pub fn new() -> RouteBuilder {
        RouteBuilder::new()
    }

    /// Create a new [`RouteBuilder`] from the current Route
    ///
    /// [`RouteBuilder`]: crate::RouteBuilder
    pub fn modify(&self) -> RouteBuilder {
        RouteBuilder {
            inner: self.inner.clone(),
        }
    }

    /// Get the next item from this route
    pub fn step(&mut self) -> Option<Address> {
        self.inner.pop_front()
    }

    /// Get the next item from this route without removing it
    pub fn next(&self) -> Option<&Address> {
        self.inner.front()
    }

    /// Get the final recipient address
    pub fn recipient(&self) -> Address {
        self.inner
            .back()
            .cloned()
            .expect("Route::recipient failed on invalid Route!")
    }
}

// Easily turn a RouteBuilder into a Route
impl From<RouteBuilder> for Route {
    fn from(RouteBuilder { inner }: RouteBuilder) -> Self {
        Self { inner }
    }
}

// A single address is a valid route
impl<T: Into<Address>> From<T> for Route {
    fn from(t: T) -> Self {
        let addr: Address = t.into();
        Route::new().append(addr).into()
    }
}

/// Utility type to build and manipulate routes
pub struct RouteBuilder {
    inner: VecDeque<Address>,
}

impl RouteBuilder {
    fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    /// Push a new item to the back of the route
    pub fn append<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_back(addr.into());
        self
    }

    /// Push a new item to the front of the route
    pub fn prepend<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_front(addr.into());
        self
    }

    /// Replace the next item in the route with a new address
    ///
    /// Similar to [`Self::prepend(...)`](RouteBuilder::prepend), but
    /// drops the previous HEAD value.
    pub fn replace<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.pop_front();
        self.inner.push_front(addr.into());
        self
    }
}
