use ockam_core::compat::boxed::Box;
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, Result, Route, TransportType};

/// Generic representation of a Transport
/// At minimum, a Transport must be able
///  - return its type
///  - instantiate workers for all the addresses with that transport type in a Route
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Return the type of the Transport
    fn transport_type(&self) -> TransportType;

    /// Instantiate transport workers for each address in the route with the transport type
    /// and replace the transport address with the local address of the transport worker
    async fn resolve_route(&self, flow_controls: &FlowControls, route: Route) -> Result<Route>;
}
