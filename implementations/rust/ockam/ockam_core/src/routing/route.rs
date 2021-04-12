use crate::{
    lib::{
        fmt::{self, Display},
        String, Vec, VecDeque,
    },
    Address, Result, RouteError,
};
use serde::{Deserialize, Serialize};

/// A full route to a peer
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Route {
    inner: VecDeque<Address>,
}

impl Route {
    /// Create an empty RouteBuilder
    pub fn new() -> RouteBuilder<'static> {
        RouteBuilder::new()
    }

    /// Parse a route from a string
    pub fn parse<S: Into<String>>(s: S) -> Option<Route> {
        let s = s.into();
        if s == "" {
            return None;
        }

        let addrs = s.split("=>").collect::<Vec<_>>();

        // Invalid route
        if addrs.len() == 0 {
            return None;
        }

        Some(
            addrs
                .into_iter()
                .fold(Route::new(), |r, addr| r.append(addr.trim()))
                .into(),
        )
    }

    /// Create a new [`RouteBuilder`] from the current Route
    ///
    /// [`RouteBuilder`]: crate::RouteBuilder
    pub fn modify(&mut self) -> RouteBuilder {
        RouteBuilder {
            inner: self.inner.clone(),
            write_back: Some(self),
        }
    }

    /// Get the next item from this route
    pub fn step(&mut self) -> Result<Address> {
        self.inner
            .pop_front()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Get the next item from this route without removing it
    pub fn next(&self) -> Result<&Address> {
        self.inner
            .front()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Get the final recipient address
    pub fn recipient(&self) -> Address {
        self.inner
            .back()
            .cloned()
            .expect("Route::recipient failed on invalid Route!")
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.inner
                .iter()
                .map(|a| format!("{}", a))
                .collect::<Vec<_>>()
                .join(" => ")
        )
    }
}

// Easily turn a RouteBuilder into a Route
impl From<RouteBuilder<'_>> for Route {
    fn from(RouteBuilder { ref inner, .. }: RouteBuilder) -> Self {
        Self {
            inner: inner.clone(),
        }
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
pub struct RouteBuilder<'r> {
    inner: VecDeque<Address>,
    write_back: Option<&'r mut Route>,
}

impl RouteBuilder<'_> {
    fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            write_back: None,
        }
    }

    /// Push a new item to the back of the route
    pub fn append<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_back(addr.into());
        self
    }

    /// Push an item with an explicit type to the back of the route
    pub fn append_t<A: Into<String>>(mut self, t: u8, addr: A) -> Self {
        self.inner
            .push_back(format!("{}#{}", t, addr.into()).into());
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

impl Drop for RouteBuilder<'_> {
    fn drop(&mut self) {
        if self.write_back.is_some() {
            **self.write_back.as_mut().unwrap() = Route {
                inner: self.inner.clone(),
            };
        }
    }
}
