use super::{Address, Command, NodeError};

use crate::WorkerHandle;
use ockam_core::Error;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Node<T> {
    sender: Sender<Command<T>>,
}

impl<T> Node<T> {
    pub fn new(sender: Sender<Command<T>>) -> Self {
        Self { sender }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        match self.sender.send(Command::stop()).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }

    pub async fn create_worker(&self, worker: WorkerHandle<T>, address: Address) {
        let create_worker_command = Command::create_worker(worker, address);

        if let Err(_ignored) = self.sender.send(create_worker_command).await {
            // TODO should `create_worker` return a Result, or should we have a global error handler?
        }
    }
}
