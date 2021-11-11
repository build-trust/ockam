use crate::{
    compat::{collections::VecDeque, string::String, vec::Vec},
    Address, Result, RouteError,
};
use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};

/// A full route to a peer
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Route {
    inner: VecDeque<Address>,
}

impl Route {
    /// Create an empty RouteBuilder
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> RouteBuilder<'static> {
        RouteBuilder::new()
    }

    /// Create a route from a Vec of addresses
    pub fn create<T: Into<Address>>(vt: Vec<T>) -> Self {
        let mut route = Route::new();
        for addr in vt {
            route = route.append(addr.into());
        }
        route.into()
    }

    /// Parse a route from a string
    pub fn parse<S: Into<String>>(s: S) -> Option<Route> {
        let s = s.into();
        if s.is_empty() {
            return None;
        }

        let addrs = s.split("=>").collect::<Vec<_>>();

        // Invalid route
        if addrs.is_empty() {
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

// A single address is a valid route. TODO Using Into<Address> here is incompatible with the From<Vec<T>> below. Why?
impl From<Address> for Route {
    fn from(address: Address) -> Self {
        let addr: Address = address;
        Route::new().append(addr).into()
    }
}

// TODO this should be covered by Into<Address>, hack for the above.
impl From<&str> for Route {
    fn from(s: &str) -> Self {
        Address::from(s).into()
    }
}

// A Vec of addresses is a valid route (if it is assumed vec index order is route order)
impl<T: Into<Address>> From<Vec<T>> for Route {
    fn from(vt: Vec<T>) -> Self {
        let mut route = Route::new();
        for t in vt {
            route = route.append(t.into());
        }
        route.into()
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

    /// Prepend a full route to an existing route
    pub fn prepend_route(mut self, route: Route) -> Self {
        route
            .inner
            .into_iter()
            .rev()
            .for_each(|addr| self.inner.push_front(addr));
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

    /// Pop front
    pub fn pop_front(mut self) -> Self {
        self.inner.pop_front();
        self
    }

    /// Pop back
    pub fn pop_back(mut self) -> Self {
        self.inner.pop_back();
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

#[cfg(test)]
mod tests {
    use crate::{Address, Error, Route, RouteError};

    fn validate_error(err: Error) {
        assert_eq!(err.domain(), RouteError::DOMAIN_NAME);
        assert_eq!(err.code(), RouteError::DOMAIN_CODE);
    }

    #[test]
    fn test_route_from_vec() {
        let address = Address::from_string("a");
        let mut route: Route = vec![address, "b".into()].into();
        assert_eq!(route.next().unwrap(), &Address::from_string("0#a"));
        assert_eq!(route.next().unwrap(), &Address::from_string("0#a"));
        assert_eq!(route.recipient(), Address::from_string("0#b"));
        assert_eq!(route.step().unwrap(), Address::from_string("0#a"));
        assert_eq!(route.step().unwrap(), Address::from_string("0#b"));
    }

    #[test]
    fn test_route_create() {
        let addresses = vec!["node-1", "node-2"];
        let route: Route = Route::create(addresses);
        assert_eq!(route.recipient(), Address::from_string("0#node-2"));
    }

    #[test]
    fn test_route_parse_empty_string() {
        assert_eq!(Route::parse(""), None);
    }

    #[test]
    fn test_route_parse_valid_input() {
        let s = " node-1 =>node-2=> node-3 ";
        let mut route = Route::parse(s).unwrap();
        assert_eq!(route.next().unwrap(), &Address::from_string("0#node-1"));
        assert_eq!(route.recipient(), Address::from_string("0#node-3"));
        let _ = route.step();
        assert_eq!(route.next().unwrap(), &Address::from_string("0#node-2"));
    }

    #[test]
    fn test_route_accessors_error_condition() {
        let s = "node-1";
        let mut route = Route::parse(s).unwrap();
        let _ = route.step();
        validate_error(route.step().err().unwrap());
        validate_error(route.next().err().unwrap());
    }

    #[test]
    #[should_panic(expected = "Route::recipient failed on invalid Route!")]
    fn test_route_no_recipient() {
        let mut route = Route::parse("node-1=>node-2").unwrap();
        let _ = route.step();
        let _ = route.step();
        route.recipient();
    }

    #[test]
    fn test_route_prepend_route() {
        let mut r1: Route = vec!["a", "b", "c"].into();
        let r2: Route = vec!["1", "2", "3"].into();

        r1.modify().prepend_route(r2);
        assert_eq!(r1, vec!["1", "2", "3", "a", "b", "c"].into());
    }
}
