use crate::{
    compat::{collections::VecDeque, string::String, vec::Vec},
    Address, AddressParseError, Result, RouteError, TransportType,
};
use core::fmt;
use serde::{Deserialize, Serialize};

/// A full route to a peer.
#[derive(Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Route {
    inner: VecDeque<Address>,
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
    ///     .try_append("bob")?
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
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
    /// # use ockam_core::{Address, Route, TransportType};
    /// # pub const TCP: TransportType = TransportType::new(1);
    /// // ["1#alice", "0#bob"]
    /// let route: Route = vec![
    ///     Address::new(TCP, "alice"),
    ///     "bob".try_into()?,
    /// ]
    /// .into();
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    ///
    pub fn create<A: Into<Address>>(vt: Vec<A>) -> Self {
        let mut route = Route::new();
        for addr in vt {
            route = route.append(addr);
        }
        route.into()
    }

    /// Like [`create`] but for any type that may be convertible into [`Address`].
    pub fn try_create<T>(vt: Vec<T>) -> Result<Self>
    where
        T: TryInto<Address>,
        T::Error: Into<crate::Error>,
    {
        let mut route = Route::new();
        for addr in vt {
            route = route.try_append(addr)?;
        }
        Ok(route.into())
    }

    /// Parse a route from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::Route;
    /// if let Ok(route) = Route::parse("1#alice => bob") {
    ///     // ["1#alice", "0#bob"]
    ///     route
    /// # ;
    /// }
    /// ```
    ///
    pub fn parse<S: AsRef<str>>(s: S) -> Result<Route> {
        if s.as_ref().is_empty() {
            return Err(RouteError::IncompleteRoute.into());
        }

        let mut r = Route::new();

        for addr in s.as_ref().split("=>") {
            r = r.try_append(addr.trim())?
        }

        if r.inner.is_empty() {
            return Err(RouteError::IncompleteRoute.into());
        }

        Ok(r.into())
    }

    /// Create a new [`RouteBuilder`] from the current `Route`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{try_route, Route};
    /// let mut route: Route = try_route!["1#alice", "bob"]?;
    ///
    /// // ["1#alice", "0#bob", "0#carol"]
    /// let route: Route = route.modify()
    ///     .try_append("carol")?
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
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
    /// # use ockam_core::{try_route, Address, Route};
    /// let mut route: Route = try_route!["1#alice", "bob"]?;
    ///
    /// // "1#alice"
    /// let next_hop: Address = route.step()?;
    ///
    /// // ["0#bob"]
    /// route;
    /// # Ok::<_, ockam_core::Error>(())
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
    /// # use ockam_core::{try_route, Address, Route};
    /// let route: Route = try_route!["1#alice", "bob"]?;
    ///
    /// // "1#alice"
    /// let next_hop: &Address = route.next()?;
    ///
    /// // ["1#alice", "0#bob"]
    /// route;
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    pub fn next(&self) -> Result<&Address> {
        self.inner
            .front()
            .ok_or_else(|| RouteError::IncompleteRoute.into())
    }

    /// Return the final recipient address.
    ///
    /// # Panics
    ///
    /// This function will panic if passed an empty route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{try_route, Address, Route};
    /// let route: Route = try_route!["1#alice", "bob"]?;
    ///
    /// // "0#bob"
    /// let final_hop: Address = route.recipient();
    ///
    /// // ["1#alice", "0#bob"]
    /// route;
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    ///
    /// `TODO` For consistency we should not panic and return a
    /// Result<&Address> instead of an Address.clone().
    pub fn recipient(&self) -> Address {
        self.inner
            .back()
            .cloned()
            .expect("Route::recipient failed on invalid Route!")
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.inner
                .iter()
                .map(|a| format!("{:?}", a))
                .collect::<Vec<_>>()
                .join(" => ")
        )
    }
}

// Convert a `RouteBuilder` into a `Route`.
impl From<RouteBuilder<'_>> for Route {
    fn from(RouteBuilder { ref inner, .. }: RouteBuilder) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

// Convert an `Address` into a `Route`.
//
// A single address can represent a valid route.
impl From<Address> for Route {
    fn from(address: Address) -> Self {
        Route::new().append(address).into()
    }
}

// Convert a `&str` into a `Route`.
//
// A string-slice reference can represent a valid route.
impl TryFrom<&str> for Route {
    type Error = AddressParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Address::try_from(s).map(Route::from)
    }
}

/// Convert a A `Vec` of `Address`es into a `Route`.
///
/// A vector of addresses can represent a valid route.
///
/// Note that this only holds if the vector index order is in the same
/// order as the expected route.
impl From<Vec<Address>> for Route {
    fn from(vt: Vec<Address>) -> Self {
        let mut route = Route::new();
        for t in vt {
            route = route.append(t);
        }
        route.into()
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
    /// # use ockam_core::{Address, Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .append(Address::local("bob"));
    ///
    /// // ["0#bob"]
    /// let route: Route = builder.into();
    /// ```
    pub fn append<A: Into<Address>>(mut self, addr: A) -> Self {
        self.inner.push_back(addr.into());
        self
    }

    /// Like [`append`] but for any type that may be convertible into an [`Address`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .try_append("1#alice")?
    ///     .try_append("bob")?;
    ///
    /// // ["1#alice, "0#bob"]
    /// let route: Route = builder.into();
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    pub fn try_append<A>(mut self, addr: A) -> Result<Self>
    where
        A: TryInto<Address>,
        A::Error: Into<crate::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.inner.push_back(a);
        Ok(self)
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

    /// Like [`prepend`] but for any type that may be convertible into an [`Address`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .try_prepend("1#alice")?
    ///     .try_prepend("0#bob")?;
    ///
    /// // ["0#bob", "1#alice"]
    /// let route: Route = builder.into();
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    pub fn try_prepend<A>(mut self, addr: A) -> Result<Self>
    where
        A: TryInto<Address>,
        A::Error: Into<crate::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.inner.push_front(a);
        Ok(self)
    }

    /// Push a new item to the front of the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{Address, Route, RouteBuilder};
    /// let builder: RouteBuilder = Route::new()
    ///     .prepend(Address::local("bob"));
    ///
    /// // ["0#bob"]
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
    /// # use ockam_core::{try_route, Route, RouteBuilder};
    /// let mut route_a: Route = try_route!["1#alice", "bob"]?;
    /// let route_b: Route = try_route!["1#carol", "dave"]?;
    ///
    /// // ["1#carol", "0#dave", "1#alice", "0#bob"]
    /// let route: Route = route_a.modify()
    ///     .prepend_route(route_b)
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
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

    /// Replace the next item in the route with a new address.
    ///
    /// Similar to [`Self::prepend(...)`](RouteBuilder::prepend), but
    /// drops the previous HEAD value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{try_route, Route, RouteBuilder};
    /// let mut route: Route = try_route!["1#alice", "bob"]?;
    ///
    /// // ["1#carol", "0#bob"]
    /// let route: Route = route.modify()
    ///     .try_replace("1#carol")?
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    pub fn try_replace<A>(mut self, addr: A) -> Result<Self>
    where
        A: TryInto<Address>,
        A::Error: Into<crate::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.inner.pop_front();
        self.inner.push_front(a);
        Ok(self)
    }

    /// Pop the front item from the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{try_route, Address, Route};
    /// let mut route: Route = try_route!["1#alice", "bob", "carol"]?;
    ///
    /// // ["0#bob", "carol"]
    /// let route: Route = route.modify()
    ///     .pop_front()
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
    /// ```
    pub fn pop_front(mut self) -> Self {
        self.inner.pop_front();
        self
    }

    /// Pop the back item from the route.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ockam_core::{try_route, Address, Route};
    /// let mut route: Route = try_route!["1#alice", "bob", "carol"]?;
    ///
    /// // ["1#alice", "0#bob"]
    /// let route: Route = route.modify()
    ///     .pop_back()
    ///     .into();
    /// # Ok::<_, ockam_core::Error>(())
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
    use crate::{Address, Error, Route};

    fn validate_error(_err: Error) {
        // assert_eq!(err.domain(), RouteError::DOMAIN_NAME);
        // assert_eq!(err.code(), RouteError::DOMAIN_CODE);
        // ???
    }

    #[test]
    fn test_route_from_vec() {
        let address = Address::try_from("a").unwrap();
        let mut route: Route = vec![address, Address::local("b")].into();
        assert_eq!(route.next().unwrap(), &Address::try_from("0#a").unwrap());
        assert_eq!(route.next().unwrap(), &Address::try_from("0#a").unwrap());
        assert_eq!(route.recipient(), Address::try_from("0#b").unwrap());
        assert_eq!(route.step().unwrap(), Address::try_from("0#a").unwrap());
        assert_eq!(route.step().unwrap(), Address::try_from("0#b").unwrap());
    }

    #[test]
    fn test_route_create() {
        let addresses = vec!["node-1", "node-2"];
        let route: Route = Route::try_create(addresses).unwrap();
        assert_eq!(route.recipient(), Address::try_from("0#node-2").unwrap());
    }

    #[test]
    fn test_route_parse_empty_string() {
        assert!(Route::parse("").is_err())
    }

    #[test]
    fn test_route_parse_valid_input() {
        let s = " node-1 =>node-2=> node-3 ";
        let mut route = Route::parse(s).unwrap();
        assert_eq!(
            route.next().unwrap(),
            &Address::try_from("0#node-1").unwrap()
        );
        assert_eq!(route.recipient(), Address::try_from("0#node-3").unwrap());
        let _ = route.step();
        assert_eq!(
            route.next().unwrap(),
            &Address::try_from("0#node-2").unwrap()
        );
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
        use crate::try_route;
        let mut r1: Route = try_route!["a", "b", "c"].unwrap();
        let r2: Route = try_route!["1", "2", "3"].unwrap();

        r1.modify().prepend_route(r2);
        assert_eq!(r1, try_route!["1", "2", "3", "a", "b", "c"].unwrap());
    }
}
