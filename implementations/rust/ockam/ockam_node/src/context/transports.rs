use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result, Route, TransportType};
use ockam_transport_core::Transport;

use crate::Context;

impl Context {
    /// Return the list of supported transports
    pub fn register_transport(&self, transport: Arc<dyn Transport>) {
        let mut transports = self.transports.write().unwrap();
        transports.insert(transport.transport_type(), transport);
    }

    /// Return true if a given transport has already been registered
    pub fn is_transport_registered(&self, transport_type: TransportType) -> bool {
        let transports = self.transports.read().unwrap();
        transports.contains_key(&transport_type)
    }

    /// For each address handled by a given transport in a route, for example, (TCP, "127.0.0.1:4000")
    /// Create a worker supporting the routing of messages for this transport and replace the address
    /// in the route with the worker address
    pub async fn resolve_transport_route(&self, route: Route) -> Result<Route> {
        let transports = self.transports.read().unwrap().clone();

        let (resolved_route, _resolved_address) =
            Self::resolve_transport_route_static(route, transports).await?;
        Ok(resolved_route)
    }

    /// For each address handled by a given transport in a route, for example, (TCP, "127.0.0.1:4000")
    /// Create a worker supporting the routing of messages for this transport and replace the address
    /// in the route with the worker address.
    /// Also, returns the connection worker address, if that connection was instantiated in this function.
    pub async fn resolve_transport_route_static(
        route: Route,
        transports: HashMap<TransportType, Arc<dyn Transport>>,
    ) -> Result<(Route, Option<Address>)> {
        // check the number of transport hops, there can be only one
        // we do this first pass over the list of addresses to avoid creating connections
        // and then having to close them if we find several hops
        // TODO: This is now possible with UDP
        let number_of_transport_hops = route.iter().filter(|a| !a.is_local()).count();
        if number_of_transport_hops > 1 {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                "only one transport hop is allowed in a route",
            ));
        }
        // return the route if there are no transport hops
        else if number_of_transport_hops == 0 {
            return Ok((route, None));
        };

        // otherwise resolve the hop address
        let mut resolved_route = Route::new();
        let mut resolved_address = None;
        for address in route.iter() {
            if !address.is_local() {
                if let Some(transport) = transports.get(&address.transport_type()) {
                    let transport_address = transport.resolve_address(address.clone()).await?;
                    resolved_address = Some(transport_address.clone());
                    resolved_route = resolved_route.append(transport_address);
                } else {
                    return Err(Error::new(
                        Origin::Transport,
                        Kind::NotFound,
                        format!("the transport is not registered for address {}", address),
                    ));
                }
            } else {
                resolved_route = resolved_route.append(address.clone());
            };
        }
        let result: Route = resolved_route.into();
        Ok((result, resolved_address))
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::{async_trait, route, Address, LOCAL};

    use super::*;

    #[ockam_macros::test(crate = "crate")]
    async fn test_transports(ctx: &mut Context) -> Result<()> {
        let transport = Arc::new(SomeTransport());
        ctx.register_transport(transport.clone());
        assert!(ctx.is_transport_registered(transport.transport_type()));
        Ok(())
    }

    #[ockam_macros::test(crate = "crate")]
    async fn test_resolve_route(ctx: &mut Context) -> Result<()> {
        let transport = Arc::new(SomeTransport());
        ctx.register_transport(transport.clone());

        // resolve a route with known transports
        let result = ctx
            .resolve_transport_route(route![(transport.transport_type(), "address")])
            .await;
        assert!(result.is_ok());

        // resolve a route with unknown transports
        let result = ctx
            .resolve_transport_route(route![(TransportType::new(1), "address")])
            .await;

        assert!(result.is_err());
        Ok(())
    }

    #[ockam_macros::test(crate = "crate")]
    async fn test_resolve_route_only_single_hop_is_allowed(ctx: &mut Context) -> Result<()> {
        let result = ctx
            .resolve_transport_route(route![
                (TransportType::new(1), "address1"),
                (LOCAL, "address2"),
                (TransportType::new(1), "address3")
            ])
            .await
            .err();

        assert!(result
            .unwrap()
            .to_string()
            .contains("only one transport hop is allowed in a route"));
        Ok(())
    }

    struct SomeTransport();

    #[async_trait]
    impl Transport for SomeTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::new(10)
        }

        /// This implementation simply marks each address as a local address
        async fn resolve_address(&self, address: Address) -> Result<Address> {
            Ok(Address::new(LOCAL, address.inner()))
        }

        async fn disconnect(&self, _address: Address) -> Result<()> {
            Ok(())
        }
    }
}
