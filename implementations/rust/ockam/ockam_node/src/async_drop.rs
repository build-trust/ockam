use crate::tokio::sync::{
    mpsc::Sender as DefaultSender,
    oneshot::{self, Receiver, Sender},
};
use crate::NodeMessage;
use ockam_core::Address;

/// A helper to implement Drop mechanisms, but async
///
/// This mechanism uses a Oneshot channel, which doesn't require an
/// async context to send into it (i.e. we can send a single message
/// from a `Drop` handler without needing to block on a future!)
///
/// The receiver is then tasked to de-allocate the specified resource.
///
/// This is not a very generic interface, i.e. it will only generate
/// stop_worker messages.  If we want to reuse this mechanism, we may
/// also want to extend the API so that other resources can specify
/// additional metadata to generate messages.
pub struct AsyncDrop {
    rx: Receiver<Address>,
    sender: DefaultSender<NodeMessage>,
}

impl AsyncDrop {
    /// Create a new AsyncDrop and AsyncDrop sender
    ///
    /// The `sender` parameter can simply be cloned from the parent
    /// Context that creates this hook, while the `address` field must
    /// refer to the address of the context that will be deallocated
    /// this way.
    pub fn new(sender: DefaultSender<NodeMessage>) -> (Self, Sender<Address>) {
        let (tx, rx) = oneshot::channel();
        (Self { rx, sender }, tx)
    }

    /// Wait for the cancellation of the channel and then send a
    /// message to the router
    ///
    /// Because this code is run detached from its original context,
    /// we can't handle any errors.
    pub async fn run(self) {
        if let Ok(addr) = self.rx.await {
            debug!("Received AsyncDrop request for address: {}", addr);

            let (msg, mut reply) = NodeMessage::stop_worker(addr, true);
            if let Err(e) = self.sender.send(msg).await {
                debug!("Failed sending AsyncDrop request to router: {}", e);
            }

            // Then check that address was properly shut down
            if reply.recv().await.is_none() {
                debug!("AsyncDrop router reply was None");
            }
        }
    }
}
