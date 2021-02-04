use crate::error::Error;
use crate::message::Message;
use crate::Context;

use ockam_core::{Result, Worker};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Node {
    sender: Sender<Message>,
}

impl Node {
    pub fn new(sender: Sender<Message>) -> Self {
        Self { sender }
    }

    pub async fn stop(&self) -> Result<()> {
        match self.sender.send(Message::stop()).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        }
    }

    /// Create and start the handler at [`Address`].
    pub async fn start_worker<S: ToString>(
        &self,
        address: S,
        worker: impl Worker<Context = Context>,
    ) -> Result<()> {
        let address = address.to_string();
        let start_worker_message = Message::start_worker(address, Box::new(worker));
        match self.sender.send(start_worker_message).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStartWorker.into()),
        }
    }
}
