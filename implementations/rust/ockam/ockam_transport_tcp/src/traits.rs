extern crate alloc;

use alloc::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

#[async_trait]
pub trait Connection {
    async fn connect(&mut self) -> Result<(), String>;
    async fn send(&mut self, message: &[u8]) -> Result<usize, String>;
    async fn receive(&mut self, message: &mut [u8]) -> Result<usize, String>;
}

#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> Result<Arc<Mutex<dyn Connection + Send>>, String>;
    fn stop(&mut self);
}
