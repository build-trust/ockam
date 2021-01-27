use super::{Command, NodeError};

use ockam_core::Error;
use std::future::Future;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Node {
    sender: Sender<Command>,
}

impl Node {
    pub fn new(sender: Sender<Command>) -> Self {
        Self { sender }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        match self.sender.send(Command::stop()).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }

    pub async fn create_worker<T>(&self, w: impl Future<Output = T> + 'static + Send)
    where
        T: Send + 'static,
    {
        // TODO: move thsi into the node executor
        tokio::spawn(w);

        match self.sender.send(Command::create_worker()).await {
            _ => (),
            // Ok(()) => Ok(()),
            // Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }
}
