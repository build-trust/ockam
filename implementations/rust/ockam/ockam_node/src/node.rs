use ockam_core::Error;
use tokio::sync::mpsc::Sender;

use crate::WorkerHandle;

use super::{Address, Command, NodeError};

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

    /// Create and start the [`WorkerHandle`] at [`Address`].
    pub async fn create_worker(&self, worker: WorkerHandle, address: Address) {
        let create_worker_command = Command::create_worker(worker, address);

        if let Err(_ignored) = self.sender.send(create_worker_command).await {
            // TODO should `create_worker` return a Result, or should we have a global error handler?
        }
    }
}
