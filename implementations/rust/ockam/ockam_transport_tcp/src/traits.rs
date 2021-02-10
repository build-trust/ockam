extern crate alloc;

use alloc::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

/// The `Connection` trait represents transport connections.
#[async_trait]
pub trait Connection {
    /// Establishes the transport connection.
    async fn connect(&mut self) -> Result<(), String>;

    /// Sends a message.
    async fn send(&mut self, message: &[u8]) -> Result<usize, String>;

    /// Receives a message.
    async fn receive(&mut self, message: &mut [u8]) -> Result<usize, String>;
}

/// The `Listerner` trait represents transport connection listeners.
#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> Result<Arc<Mutex<dyn Connection + Send>>, String>;
}
