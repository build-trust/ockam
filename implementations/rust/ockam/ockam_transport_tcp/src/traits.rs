extern crate alloc;
use crate::error::Error;
use async_trait::async_trait;

/// The `Connection` trait represents transport connections.
#[async_trait]
pub trait Connection {
    /// Establishes the transport connection.
    async fn connect(&mut self) -> Result<(), Error>;

    /// Sends a message.
    async fn send(&mut self, message: &[u8]) -> Result<usize, Error>;

    /// Receives a message.
    async fn receive(&mut self, message: &mut [u8]) -> Result<usize, Error>;
}

/// The `Listerner` trait represents transport connection listeners.
#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> Result<Box<dyn Connection + Send>, Error>;
}
