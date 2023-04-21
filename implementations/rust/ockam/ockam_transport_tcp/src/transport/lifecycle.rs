use crate::{TcpConnectionOptions, TcpRegistry, TcpTransport, TCP};
use core::str::FromStr;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, AsyncTryClone, Error, Result, Route, TransportType};
use ockam_node::Context;
use ockam_transport_core::Transport;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::debug;

impl TcpTransport {
    /// Create a TCP transport
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create(ctx: &Context) -> Result<Self> {
        let tcp = Self {
            ctx: ctx.async_try_clone().await?,
            registry: TcpRegistry::default(),
        };
        // make the TCP transport available in the list of supported transports for
        // later address resolution when socket addresses will need to be instantiated as TCP
        // worker addresses
        ctx.register_transport(Arc::new(tcp.async_try_clone().await?));
        Ok(tcp)
    }
}

impl TcpTransport {
    /// Getter
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
    /// Registry of all active connections
    pub fn registry(&self) -> &TcpRegistry {
        &self.registry
    }
}

#[async_trait]
impl Transport for TcpTransport {
    fn transport_type(&self) -> TransportType {
        TCP
    }

    async fn resolve_route(&self, flow_controls: &FlowControls, route: Route) -> Result<Route> {
        let mut result = Route::new();
        let mut number_of_tcp_hops = 0;

        for address in route.iter() {
            if address.transport_type() == TCP {
                if number_of_tcp_hops >= 1 {
                    return Err(Error::new(
                        Origin::Transport,
                        Kind::Invalid,
                        "only one TCP hop is allowed in a route",
                    ));
                }

                let options = if SocketAddr::from_str(address.address())
                    .map(|socket_addr| socket_addr.ip().is_loopback())
                    .is_ok()
                {
                    // TODO: Enable FlowControl for loopback addresses as well
                    TcpConnectionOptions::insecure()
                } else {
                    let id = flow_controls.generate_id();
                    TcpConnectionOptions::as_producer(flow_controls, &id)
                };

                number_of_tcp_hops += 1;
                let addr = self.connect(address.address().to_string(), options).await?;
                result = result.append(addr)
            } else {
                result = result.append(address.clone());
            }
        }

        let resolved = result.into();
        debug!("resolved route {} to {}", route, resolved);
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::{route, TransportType, LOCAL};
    use ockam_transport_core::TransportError;
    use std::net::TcpListener;

    #[ockam_macros::test]
    async fn test_resolve_route(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_address = "127.0.0.1:5001";
        let _listener = TcpListener::bind(tcp_address).map_err(TransportError::from)?;
        let initial_workers = ctx.list_workers().await?;

        let other_transport_type = TransportType::new(10);
        let other_address = (other_transport_type, "other_address");
        let route = tcp
            .resolve_route(
                &FlowControls::default(),
                route![(TCP, tcp_address), other_address],
            )
            .await?;

        // there are 2 additional workers
        let mut additional_workers = ctx.list_workers().await?;
        additional_workers.retain(|w| !initial_workers.contains(w));
        assert_eq!(additional_workers.len(), 2);

        // the TCP address is replaced with the TCP sender worker address
        // the other address is left unchanged
        assert_eq!(
            route.iter().map(|a| a.transport_type()).collect::<Vec<_>>(),
            vec![LOCAL, other_transport_type]
        );

        let first_address = route.next()?;
        assert!(additional_workers.contains(first_address));

        // trying to resolve the address a second time should still work
        let _route = tcp
            .resolve_route(
                &FlowControls::default(),
                route![(TCP, tcp_address), other_address],
            )
            .await?;

        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn test_resolve_route_only_single_hop_is_allowed(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_address = "127.0.0.1:5002";
        let _listener = TcpListener::bind(tcp_address).map_err(TransportError::from)?;

        let result = tcp
            .resolve_route(
                &FlowControls::default(),
                route![
                    (TCP, tcp_address),
                    (TransportType::new(10), "other_address"),
                    (TCP, tcp_address)
                ],
            )
            .await
            .err();

        assert_eq!(
            result.unwrap().to_string(),
            "only one TCP hop is allowed in a route"
        );
        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn test_resolve_route_with_dns_address(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let result = tcp
            .resolve_route(
                &FlowControls::default(),
                route![(TCP, "www.google.com:80"),],
            )
            .await
            .is_ok();

        assert!(result);
        ctx.stop().await
    }
}
