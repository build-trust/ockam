extern crate alloc;
use crate::error::TransportError;
use async_trait::async_trait;

/// The `Connection` trait represents transport connections.
#[async_trait]
pub trait Connection {
    /// Establishes the transport connection. For connections-oriented
    /// transports suchs as TCP, blocks until a connection with the remote
    /// is established. For connectionless transports such as UDP, returns immediately.
    /// # Examples
    /// ```
    /// use crate::ockam_transport_tcp::connection::TcpConnection;
    /// use std::str::FromStr;
    /// let mut connection =
    /// TcpConnection::create(std::net::SocketAddr::from_str(&address).unwrap());
    /// let r = connection.connect().await;
    /// ```
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Sends a message.
    async fn send(&mut self, message: &[u8]) -> Result<usize, TransportError>;

    /// Receives a message.
    async fn receive(&mut self, message: &mut [u8]) -> Result<usize, TransportError>;
}

/// The `Listener` trait represents transport connection listeners.
#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> Result<Box<dyn Connection + Send>, TransportError>;
}
