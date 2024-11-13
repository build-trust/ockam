use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, Error, Result, TransportType, TryClone};
use ockam_node::Context;
use ockam_transport_core::Transport;
use std::sync::Arc;
use tracing::instrument;

use crate::UdpBindArguments;
use crate::{UdpBindOptions, UdpTransport, UDP};

impl UdpTransport {
    /// Create a UDP transport
    ///
    /// ```rust
    /// use ockam_transport_udp::UdpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let udp = UdpTransport::create(&ctx)?;
    /// # Ok(()) }
    /// ```
    #[instrument(name = "create udp transport", skip_all)]
    pub fn create(ctx: &Context) -> Result<Self> {
        let udp = Self {
            ctx: Arc::new(ctx.try_clone()?),
        };
        // make the UDP transport available in the list of supported transports for
        // later address resolution when socket addresses will need to be instantiated as UDP
        // worker addresses
        ctx.register_transport(Arc::new(udp.clone()));
        Ok(udp)
    }
}

impl UdpTransport {
    /// Getter
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

#[async_trait]
impl Transport for UdpTransport {
    fn transport_type(&self) -> TransportType {
        UDP
    }

    async fn resolve_address(&self, address: &Address) -> Result<Address> {
        if address.transport_type() == UDP {
            Ok(self
                .bind(
                    UdpBindArguments::new()
                        .with_peer_address(address.address())
                        .await?,
                    UdpBindOptions::new(),
                )
                .await?
                .into())
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::NotFound,
                format!(
                    "this address can not be resolved by a UDP transport {}",
                    address
                ),
            ))
        }
    }

    fn disconnect(&self, address: &Address) -> Result<()> {
        self.unbind(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_transport_core::TransportError;
    use std::time::Duration;
    use tokio::net::UdpSocket;

    #[ockam_macros::test]
    async fn test_resolve_address(ctx: &mut Context) -> Result<()> {
        let udp = UdpTransport::create(ctx)?;
        let udp_address = "127.0.0.1:0";
        let initial_workers = ctx.list_workers()?;
        let socket = UdpSocket::bind(udp_address)
            .await
            .map_err(TransportError::from)?;
        let socket_address = socket.local_addr().unwrap().to_string();

        let resolved = udp
            .resolve_address(&Address::new_with_string(UDP, socket_address.clone()))
            .await?;

        // there are 2 additional workers
        let mut additional_workers = ctx.list_workers()?;
        additional_workers.retain(|w| !initial_workers.contains(w));
        assert_eq!(additional_workers.len(), 2);

        // the UDP address is replaced with the UDP sender worker address
        assert!(additional_workers.contains(&resolved));

        // trying to resolve the address a second time should still work
        let _route = udp
            .resolve_address(&Address::new_with_string(UDP, socket_address))
            .await?;

        tokio::time::sleep(Duration::from_millis(250)).await;

        Ok(())
    }

    #[ockam_macros::test]
    async fn test_resolve_route_with_dns_address(ctx: &mut Context) -> Result<()> {
        let udp = UdpTransport::create(ctx)?;
        let udp_address = "127.0.0.1:0";
        let socket = UdpSocket::bind(udp_address)
            .await
            .map_err(TransportError::from)?;
        let socket_address = socket.local_addr().unwrap();

        let result = udp
            .resolve_address(&Address::new_with_string(
                UDP,
                format!("localhost:{}", socket_address.port()),
            ))
            .await;
        assert!(result.is_ok());

        tokio::time::sleep(Duration::from_millis(250)).await;

        Ok(())
    }
}
