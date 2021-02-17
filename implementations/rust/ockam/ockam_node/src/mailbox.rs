use ockam_core::Encoded;

use tokio::sync::mpsc::{Receiver, Sender, channel};

/// A mailbox for encoded messages
///
/// Message type information can't be exposed at this stage because
/// they need to either be typed in the [`Relay`](crate::Relay) or in
/// the worker's [`Context`](crate::Context).
#[derive(Debug)]
pub struct Mailbox {
    rx: Receiver<Encoded>,
    tx: Sender<Encoded>,
}

impl Mailbox {
    pub fn new(rx: Receiver<Encoded>, tx: Sender<Encoded>) -> Self {
        Self { rx, tx }
    }

    pub(crate) fn fake() -> Self {
        let (tx, rx) = channel(1);
        Self { tx, rx }
    }
    
    pub fn sender(&self) -> Sender<Encoded> {
        self.tx.clone()
    }

    /// Get the next message from the mailbox
    pub async fn next(&mut self) -> Option<Encoded> {
        self.rx.recv().await
    }

    /// If a message wasn't expected, requeue it
    pub async fn requeue(&self, msg: Encoded) {
        self.tx.send(msg).await.unwrap();
    }
}
