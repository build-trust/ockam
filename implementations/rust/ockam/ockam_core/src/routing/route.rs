use crate::{
    compat::{collections::VecDeque, string::String, vec::Vec},
    Address, Result, RouteError, TransportType,
};
use core::fmt::{self, Display};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// A full route to a peer.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Route {
    #[n(1)] inner: VecDeque<Address>,
}

impl Route {
    /// Create an empty [`RouteBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, TransportType};
    /// # pub const TCP: TransportType = TransportType::new(1);
    /// // ["1#alice", "0#bob"]
    /// let route: Route = Route::new()
    ///     .append_t(TCP, "alice")
    ///     .append("bob")
    ///     .into();
    /// ```
    ///
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> RouteBuilder<'static> {
        RouteBuilder::new()
    }

    /// Create a route from a `Vec` of [`Address`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Address, route, TransportType};
    /// # pub const TCP: TransportType = TransportType::new(1);
    /// // ["1#alice", "0#bob"]
    /// let route = route![
    ///     Address::new(TCP, "alice"),
    ///     "bob",
    /// ];
    /// ```
    ///
    pub fn create<T: Into<Address>>(vt: Vec<T>) -> Self {
        let mut route = Route::new();
        for addr in vt {
            route = route.append(addr.into());
        }
        route.into()
    }

    /// Parse a route from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::Route;
    /// if let Some(route) = Route::parse("1#alice => bob") {
    ///     // ["1#alice", "0#bob"]
    ///     route
    /// # ;
    /// }
    /// ```
    ///
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

    /// Create a new [`RouteBuilder`] from the current `Route`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Route};
    /// let mut route: Route = route!["1#alice", "bob"];
    ///
    /// // ["1#alice", "0#bob", "0#carol"]
    /// let route: Route = route.modify()
    ///     .append("carol")
    ///     .into();
    /// ```
    ///
    pub fn modify(&mut self) -> RouteBuilder {
        RouteBuilder {
            inner: self.inner.clone(),
            write_back: Some(self),
        }
    }

    /// Return the next `Address` and remove it from this route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Address, Result, Route};
    /// # fn main() -> Result<()> {
    /// let mut route: Route = route!["1#alice", "bob"];
    ///
    /// // "1#alice"
    /// let next_hop: Address = route.step()?;
    ///
    /// // ["0#bob"]
    /// route
    /// # ;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    pub fn step(&mut self) -> Result<Address> {
        self.inner
            .pop_front()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Return the next `Address` from this route without removing it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Address, Result, Route};
    /// # fn main() -> Result<()> {
    /// let route: Route = route!["1#alice", "bob"];
    ///
    /// // "1#alice"
    /// let next_hop: &Address = route.next()?;
    ///
    /// // ["1#alice", "0#bob"]
    /// route
    /// # ;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    pub fn next(&self) -> Result<&Address> {
        self.inner
            .front()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Return the final recipient address.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Address, Result, Route};
    /// # fn main() -> Result<()> {
    /// let route: Route = route!["1#alice", "bob"];
    ///
    /// // "0#bob"
    /// let final_hop: Address = route.recipient()?;
    ///
    /// // ["1#alice", "0#bob"]
    /// route
    /// # ;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn recipient(&self) -> Result<Address> {
        // `TODO` For consistency we should return a
        // Result<&Address> instead of an Address.clone().
        self.inner
            .back()
            .cloned()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Iterate over all addresses of this route.
    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.inner.iter()
    }

    /// Number of hops.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the route is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if route contains `needle`.
    ///
    /// Returns `Err(_)` if `needle` is an empty route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Route, Result};
    /// # fn main() -> Result<()> {
    /// let r: Route = route!["a", "b", "c", "d"];
    ///
    /// // true
    /// let res = r.contains_route(&route!["b", "c"])?;
    ///
    /// // false
    /// let res = r.contains_route(&route!["a", "c"])?;
    ///
    /// // false
    /// let res = r.contains_route(&route!["a", "b", "c", "d", "e"])?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn contains_route(&self, needle: &Route) -> Result<bool> {
        if needle.is_empty() {
            return Err(RouteError::IncompleteRoute.into());
        }

        let hl = self.len();
        let nl = needle.len();
        if nl > hl {
            return Ok(false);
        }

        // The below uses many iterators.
        // An alternative might be to use `VecDeque::make_contiguous()` and slices.
        for i in 0..=(hl - nl) {
            let tmp = self.inner.iter().skip(i).take(nl);
            if tmp.eq(needle.iter()) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return true if all the addresses composing that route are local addresses
    pub fn is_local(&self) -> bool {
        self.iter().all(|a| a.is_local())
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

/// Convert a `RouteBuilder` into a `Route`.
impl From<RouteBuilder<'_>> for Route {
    fn from(RouteBuilder { ref inner, .. }: RouteBuilder) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

/// Convert an `Address` into a `Route`.
///
/// A single address can represent a valid route.
impl<T: Into<Address>> From<T> for Route {
    fn from(addr: T) -> Self {
        Route::create(vec![addr])
    }
}

/// A utility type for building and manipulating routes.
pub struct RouteBuilder<'r> {
    inner: VecDeque<Address>,
    write_back: Option<&'r mut Route>,
}

impl Default for RouteBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteBuilder<'_> {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            write_back: None,
        }
    }

    /// Push a new item to the back of the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .append("1#alice")
    ///     .append("bob");
    ///
    /// // ["1#alice, "0#bob"]
    /// let route: Route = builder.into();
    /// ```
    ///
    pub fn append<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_back(addr.into());
        self
    }

    /// Push an item with an explicit type to the back of the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, RouteBuilder, TransportType, LOCAL};
    /// # pub const TCP: TransportType = TransportType::new(1);
    /// let builder: RouteBuilder = Route::new()
    ///     .append_t(TCP, "alice")
    ///     .append_t(LOCAL, "bob");
    ///
    /// // ["1#alice", "0#bob"]
    /// let route: Route = builder.into();
    /// ```
    ///
    pub fn append_t<A: Into<String>>(mut self, ty: TransportType, addr: A) -> Self {
        self.inner.push_back(Address::from((ty, addr.into())));
        self
    }

    /// Push a new item to the front of the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .prepend("1#alice")
    ///     .prepend("0#bob");
    ///
    /// // ["0#bob", "1#alice"]
    /// let route: Route = builder.into();
    /// ```
    ///
    pub fn prepend<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_front(addr.into());
        self
    }

    /// Prepend a full route to an existing route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Route, RouteBuilder};
    /// let mut route_a: Route = route!["1#alice", "bob"];
    /// let route_b: Route = route!["1#carol", "dave"];
    ///
    /// // ["1#carol", "0#dave", "1#alice", "0#bob"]
    /// let route: Route = route_a.modify()
    ///     .prepend_route(route_b)
    ///     .into();
    /// ```
    ///
    pub fn prepend_route(mut self, route: Route) -> Self {
        route
            .inner
            .into_iter()
            .rev()
            .for_each(|addr| self.inner.push_front(addr));
        self
    }

    /// Append a full route to an existing route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Route, RouteBuilder};
    /// let mut route_a: Route = route!["1#alice", "bob"];
    /// let route_b: Route = route!["1#carol", "dave"];
    ///
    /// // ["1#alice", "0#bob", "1#carol", "0#dave"]
    /// let route: Route = route_a.modify()
    ///     .append_route(route_b)
    ///     .into();
    /// ```
    ///
    pub fn append_route(mut self, route: Route) -> Self {
        route
            .inner
            .into_iter()
            .for_each(|addr| self.inner.push_back(addr));
        self
    }

    /// Replace the next item in the route with a new address.
    ///
    /// Similar to [`Self::prepend(...)`](RouteBuilder::prepend), but
    /// drops the previous HEAD value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Route, RouteBuilder};
    /// let mut route: Route = route!["1#alice", "bob"];
    ///
    /// // ["1#carol", "0#bob"]
    /// let route: Route = route.modify()
    ///     .replace("1#carol")
    ///     .into();
    /// ```
    ///
    pub fn replace<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.pop_front();
        self.inner.push_front(addr.into());
        self
    }

    /// Pop the front item from the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Address, Route};
    /// let mut route: Route = route!["1#alice", "bob", "carol"];
    ///
    /// // ["0#bob", "carol"]
    /// let route: Route = route.modify()
    ///     .pop_front()
    ///     .into();
    /// ```
    ///
    pub fn pop_front(mut self) -> Self {
        self.inner.pop_front();
        self
    }

    /// Pop the back item from the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{route, Address, Route};
    /// let mut route: Route = route!["1#alice", "bob", "carol"];
    ///
    /// // ["1#alice", "0#bob"]
    /// let route: Route = route.modify()
    ///     .pop_back()
    ///     .into();
    /// ```
    ///
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
    use crate::{route, Address, Error, Route};

    fn validate_error(_err: Error) {
        // assert_eq!(err.domain(), RouteError::DOMAIN_NAME);
        // assert_eq!(err.code(), RouteError::DOMAIN_CODE);
        // ???
    }

    #[test]
    fn test_route_macro() {
        let address = Address::from_string("a");
        let mut route = route![address, "b"];
        assert_eq!(route.next().unwrap(), &Address::from_string("0#a"));
        assert_eq!(route.next().unwrap(), &Address::from_string("0#a"));
        assert_eq!(route.recipient().unwrap(), Address::from_string("0#b"));
        assert_eq!(route.step().unwrap(), Address::from_string("0#a"));
        assert_eq!(route.step().unwrap(), Address::from_string("0#b"));
    }

    #[test]
    fn test_route_create() {
        let addresses = vec!["node-1", "node-2"];
        let route = Route::create(addresses);
        assert_eq!(route.recipient().unwrap(), Address::from_string("0#node-2"));
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
        assert_eq!(route.recipient().unwrap(), Address::from_string("0#node-3"));
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
    fn test_route_no_recipient() -> Result<(), ()> {
        let mut route = Route::parse("node-1=>node-2").unwrap();
        let _ = route.step();
        let _ = route.step();
        match route.recipient() {
            Ok(_) => Err(()),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn test_route_prepend_route() {
        let mut r1 = route!["a", "b", "c"];
        let r2 = route!["1", "2", "3"];

        r1.modify().prepend_route(r2);
        assert_eq!(r1, route!["1", "2", "3", "a", "b", "c"]);
    }

    #[test]
    fn test_route_contains_route() {
        let r = route!["a", "b", "c", "d", "e"];

        assert!(matches!(r.contains_route(&route!["a"]), Ok(true)));
        assert!(matches!(r.contains_route(&route!["a", "b"]), Ok(true)));
        assert!(matches!(r.contains_route(&route!["b", "c"]), Ok(true)));
        assert!(matches!(r.contains_route(&route!["c", "d"]), Ok(true)));
        assert!(matches!(r.contains_route(&route!["e"]), Ok(true)));

        assert!(matches!(r.contains_route(&route![]), Err(_)));

        assert!(matches!(
            r.contains_route(&route!["a", "b", "c", "d", "e", "f"]),
            Ok(false)
        ));

        assert!(matches!(r.contains_route(&route!["a", "c"]), Ok(false)));
        assert!(matches!(r.contains_route(&route!["x"]), Ok(false)));
    }
}
