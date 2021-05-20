use crate::{lib::String, Address, Route};

/// A utility structure for building routes to remote services
///
/// This type SHOULD be returned by any transport implementation that
/// uses special address types.
pub struct ServiceBuilder {
    base: Route,
}

impl ServiceBuilder {
    /// Create a new service builder
    pub fn new(tt: u8, base: String) -> Self {
        Self {
            base: Route::new().append_t(tt, base).into(),
        }
    }

    /// Create a route to a service
    pub fn service<A: Into<Address>>(&self, addr: A) -> Route {
        self.base.clone().modify().append(addr.into()).into()
    }
}
