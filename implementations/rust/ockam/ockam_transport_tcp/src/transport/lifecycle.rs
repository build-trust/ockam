use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, AsyncTryClone, Error, Result, TransportType};
use ockam_node::Context;
use ockam_transport_core::Transport;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::instrument;

use crate::{TcpConnectionOptions, TcpListenerInfo, TcpRegistry, TcpSenderInfo, TcpTransport, TCP};

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
    #[instrument(name = "create tcp transport", skip_all)]
    pub async fn create(ctx: &Context) -> Result<Self> {
        let tcp = Self {
            ctx: Arc::new(ctx.async_try_clone().await?),
            registry: TcpRegistry::default(),
        };
        // make the TCP transport available in the list of supported transports for
        // later address resolution when socket addresses will need to be instantiated as TCP
        // worker addresses
        ctx.register_transport(Arc::new(tcp.clone()));
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

    /// Search for a connection with the provided socket address
    pub fn find_connection_by_socketaddr(
        &self,
        socket_address: SocketAddr,
    ) -> Option<TcpSenderInfo> {
        self.registry()
            .get_all_sender_workers()
            .into_iter()
            .find(|x| x.socket_address() == socket_address)
    }

    /// Search for a connection with the provided address
    pub fn find_connection(&self, address: String) -> Option<TcpSenderInfo> {
        match address.parse::<SocketAddr>() {
            Ok(socket_address) => self.find_connection_by_socketaddr(socket_address),
            Err(_err) => {
                let address: Address = address.into();

                // Check if it's a Receiver Address
                let address = if let Some(receiver) = self
                    .registry()
                    .get_all_receiver_processors()
                    .into_iter()
                    .find(|x| x.address() == &address)
                {
                    receiver.sender_address().clone()
                } else {
                    address
                };

                self.registry()
                    .get_all_sender_workers()
                    .into_iter()
                    .find(|x| x.address() == &address)
            }
        }
    }

    /// Search for a listener with the provided socket address
    pub fn find_listener_by_socketaddress(
        &self,
        socket_address: SocketAddr,
    ) -> Option<TcpListenerInfo> {
        self.registry()
            .get_all_listeners()
            .into_iter()
            .find(|x| x.socket_address() == socket_address)
    }

    /// Search for a listener with the provided address
    pub fn find_listener(&self, address: String) -> Option<TcpListenerInfo> {
        match address.parse::<SocketAddr>() {
            Ok(socket_address) => self.find_listener_by_socketaddress(socket_address),
            Err(_err) => {
                let address: Address = address.into();

                self.registry()
                    .get_all_listeners()
                    .into_iter()
                    .find(|x| x.address() == &address)
            }
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    fn transport_type(&self) -> TransportType {
        TCP
    }

    async fn resolve_address(&self, address: Address) -> Result<Address> {
        if address.transport_type() == TCP {
            Ok(self
                .connect(address.address().to_string(), TcpConnectionOptions::new())
                .await?
                .into())
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::NotFound,
                format!(
                    "this address can not be resolved by a TCP transport {}",
                    address
                ),
            ))
        }
    }

    async fn disconnect(&self, address: Address) -> Result<()> {
        self.disconnect(address).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_transport_core::TransportError;
    use std::time::Duration;
    use tokio::net::TcpListener;

    #[ockam_macros::test]
    async fn test_resolve_address(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_address = "127.0.0.1:0";
        let initial_workers = ctx.list_workers().await?;
        let listener = TcpListener::bind(tcp_address)
            .await
            .map_err(TransportError::from)?;
        let local_address = listener.local_addr().unwrap().to_string();

        tokio::spawn(async move {
            // Accept two connections, sleep for 100ms and quit
            let (_stream1, _) = listener.accept().await.unwrap();
            let (_stream2, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let resolved = tcp
            .resolve_address(Address::new_with_string(TCP, local_address.clone()))
            .await?;

        // there are 2 additional workers
        let mut additional_workers = ctx.list_workers().await?;
        additional_workers.retain(|w| !initial_workers.contains(w));
        assert_eq!(additional_workers.len(), 2);

        // the TCP address is replaced with the TCP sender worker address
        assert!(additional_workers.contains(&resolved));

        // trying to resolve the address a second time should still work
        let _route = tcp
            .resolve_address(Address::new_with_string(TCP, local_address))
            .await?;

        tokio::time::sleep(Duration::from_millis(250)).await;

        Ok(())
    }

    #[ockam_macros::test]
    async fn test_resolve_route_with_dns_address(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_address = "127.0.0.1:0";
        let listener = TcpListener::bind(tcp_address)
            .await
            .map_err(TransportError::from)?;
        let socket_address = listener.local_addr().unwrap();

        tokio::spawn(async move {
            // Accept two connections, sleep for 100ms and quit
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let result = tcp
            .resolve_address(Address::new_with_string(
                TCP,
                format!("localhost:{}", socket_address.port()),
            ))
            .await;
        assert!(result.is_ok());

        tokio::time::sleep(Duration::from_millis(250)).await;

        Ok(())
    }
}
