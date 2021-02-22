use async_trait::async_trait;
use ockam_router::message::{RouterAddress, RouterMessage};

/// The `Connection` trait represents transport connections.
#[async_trait]
pub trait Connection: Send + 'static {
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
    async fn connect(&mut self) -> ockam::Result<()>;

    /// Sends a u8 buffer.
    async fn send(&mut self, buff: &[u8]) -> ockam::Result<usize>;

    /// Receives a u8 buffer.
    async fn receive(&mut self, buff: &mut [u8]) -> ockam::Result<usize>;

    /// Sends a RouterMessage.
    async fn send_message(&mut self, msg: RouterMessage) -> ockam::Result<usize>;

    /// Receives a RouterMessage.
    async fn receive_message(&mut self) -> ockam::Result<RouterMessage>;

    fn get_local_address(&self) -> RouterAddress;
    fn get_remote_address(&self) -> RouterAddress;
}

/// The `Listener` trait represents transport connection listeners.
#[async_trait]
pub trait Listener: Send + 'static {
    async fn accept(&mut self) -> ockam::Result<Box<dyn Connection + Send>>;
}
