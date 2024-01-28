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
