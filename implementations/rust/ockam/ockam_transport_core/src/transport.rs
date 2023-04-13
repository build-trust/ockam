use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Address, Result, TransportType};

/// Generic representation of a Transport
/// At minimum, a Transport must be able
///  - return its type
///  - instantiate workers for all the addresses with that transport type in a Route
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Return the type of the Transport
    fn transport_type(&self) -> TransportType;

    /// Instantiate transport workers for in order to communicate with a remote address
    /// and return the local address of the transport worker
    async fn resolve_address(&self, address: Address) -> Result<Address>;
}
