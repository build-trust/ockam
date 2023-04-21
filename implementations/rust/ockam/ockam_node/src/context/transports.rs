use crate::Context;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControls;
use ockam_core::{Result, Route, TransportType};
use ockam_transport_core::Transport;

impl Context {
    /// Return the list of supported transports
    pub fn register_transport(&self, transport: Arc<dyn Transport>) {
        let mut transports = self.transports.lock().unwrap();
        transports.insert(transport.transport_type(), transport);
    }

    /// Return true if a given transport has already been registered
    pub fn is_transport_registered(&self, transport_type: TransportType) -> bool {
        let transports = self.transports.lock().unwrap();
        transports.contains_key(&transport_type)
    }

    /// For each address handled by a given transport in a route, for example, (TCP, "127.0.0.1:4000")
    /// Create a worker supporting the routing of messages for this transport and replace the address
    /// in the route with the worker address
    pub async fn resolve_transport_route(
        &self,
        flow_controls: &FlowControls,
        route: Route,
    ) -> Result<Route> {
        let transports = self.transports.lock().unwrap().clone();
        let mut resolved = route;
        for transport in transports.values() {
            resolved = transport.resolve_route(flow_controls, resolved).await?;
        }
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::async_trait;

    #[ockam_macros::test(crate = "crate")]
    async fn test_transports(ctx: &mut Context) -> Result<()> {
        let transport = Arc::new(SomeTransport());
        ctx.register_transport(transport.clone());
        assert!(ctx.is_transport_registered(transport.transport_type()));
        ctx.stop().await
    }

    struct SomeTransport();

    #[async_trait]
    impl Transport for SomeTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::new(0)
        }

        async fn resolve_route(
            &self,
            _flow_controls: &FlowControls,
            route: Route,
        ) -> Result<Route> {
            Ok(route)
        }
    }
}
