use crate::connection::TcpConnection;
use async_trait::async_trait;
use ockam_router::message::TransportMessage;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ConnectionMessage {
    Connect,
    SendMessage(TransportMessage),
    ReceiveMessage,
    Alive,
}

/// The `Listener` trait represents transport connection listeners.
#[async_trait]
pub trait Listener: Send + 'static {
    async fn accept(&mut self) -> ockam::Result<Box<TcpConnection>>;
}
