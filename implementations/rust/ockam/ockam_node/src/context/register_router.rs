use crate::Context;
use ockam_core::{Address, Result, TransportType};

impl Context {
    // TODO: This method should be deprecated
    /// Register a router for a specific address type
    pub fn register<A: Into<Address>>(&self, type_: TransportType, addr: A) -> Result<()> {
        self.router()?.register_router(type_, addr.into())
    }
}
