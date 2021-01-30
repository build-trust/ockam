use super::{Address, Command, NodeError};
use ockam_core::Error;
use std::any::Any;
use tokio::sync::mpsc::Sender;

/// The Ockam Node API.
#[derive(Clone, Debug)]
pub struct Node {
    sender: Sender<Command>,
}

impl Node {
    /// Create a new [`Node`].
    pub fn new(sender: Sender<Command>) -> Self {
        Self { sender }
    }

    /// Stop the [`Node`].
    pub async fn stop(&self) -> Result<(), Error> {
        match self.sender.send(Command::stop()).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }

    /// Create and start the handler at [`Address`].
    pub async fn start_worker<T>(&self, handler: T, address: Address)
    where
        T: Any + Send,
    {
        let create_worker_command = Command::create_worker(Box::new(handler), address);
        if let Err(_ignored) = self.sender.send(create_worker_command).await {
            // TODO should `create_worker` return a Result, or should we have a global error handler?
        }
    }
}
