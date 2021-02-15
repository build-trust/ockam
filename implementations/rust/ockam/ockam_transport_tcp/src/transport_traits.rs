use crate::error::TransportError;
use async_trait::async_trait;

/// The `Connection` trait represents transport connections.
#[async_trait]
pub trait Connection {
    /// Establishes the transport connection. For connections-oriented
    /// transports suchs as TCP, blocks until a connection with the remote
    /// is established. For connectionless transports such as UDP, returns immediately.
    /// # Examples
    /// ```ignore
    /// use crate::ockam_transport_tcp::connection::TcpConnection;
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// let address = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    /// let mut connection = TcpConnection::new(address).await.unwrap();
    /// let r = connection.connect().await.unwrap();
    /// ```
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Sends a u8 buffer.
    async fn send(&mut self, buff: &[u8]) -> Result<usize, TransportError>;

    /// Receives a u8 buffer.
    async fn receive(&mut self, buff: &mut [u8]) -> Result<usize, TransportError>;
}

/// The `Listener` trait represents transport connection listeners.
#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> Result<Box<dyn Connection + Send>, TransportError>;
}
