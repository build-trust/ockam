use crate::compat::asynchronous::RwLock;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::format;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_executor::channel::{self, Receiver, Sender};

/// This broadcast channel stores a list of sender who expect to receive a message of type T
#[derive(Clone)]
pub struct BroadcastChannel<T> {
    subscribers: Vec<Sender<T>>,
}

/// Restricted interface for sending a message
pub struct BroadcastSender<T> {
    channel: Arc<RwLock<BroadcastChannel<T>>>,
}

impl<T: Clone + Debug> BroadcastSender<T> {
    /// Create a new broadcast sender
    pub fn new(channel: Arc<RwLock<BroadcastChannel<T>>>) -> BroadcastSender<T> {
        BroadcastSender { channel }
    }

    /// Send a message to all subscribers
    pub async fn send(&self, msg: T) -> ockam_core::Result<()> {
        let channel = self.channel.read().await;
        channel.broadcast(msg).await
    }
}

/// Restricted interface for receiving a message
pub struct BroadcastReceiver<T> {
    channel: Arc<RwLock<BroadcastChannel<T>>>,
}

impl<T: Clone + Debug> BroadcastReceiver<T> {
    /// Create a new broadcast receiver
    pub fn new(channel: Arc<RwLock<BroadcastChannel<T>>>) -> BroadcastReceiver<T> {
        BroadcastReceiver { channel }
    }

    /// Wait until a new message is received
    pub async fn receive(&self) -> ockam_core::Result<T> {
        let mut receiver = {
            let mut channel = self.channel.write().await;
            channel.subscribe()
        };
        receiver.recv().await.ok_or_else(|| {
            ockam_core::Error::new(
                Origin::Channel,
                Kind::Internal,
                "cannot receive a message over a broadcast channel",
            )
        })
    }
}

/// Create a new broadcast channel
pub fn channel<T: Clone + Debug>() -> BroadcastChannel<T> {
    BroadcastChannel::new()
}

impl<T: Clone + Debug> Default for BroadcastChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Debug> BroadcastChannel<T> {
    /// Create a new broadcast channel
    pub fn new() -> Self {
        BroadcastChannel {
            subscribers: Vec::new(),
        }
    }

    /// Subscribe to messages
    pub fn subscribe(&mut self) -> Receiver<T> {
        let (sender, receiver) = channel::channel(1);
        self.subscribers.push(sender);
        receiver
    }

    /// Send a message to all subscribers
    pub async fn broadcast(&self, msg: T) -> ockam_core::Result<()> {
        for sender in &self.subscribers {
            sender.send(msg.clone()).await.map_err(|e| {
                ockam_core::Error::new(
                    Origin::Channel,
                    Kind::Internal,
                    format!("cannot send a message over a broadcast channel {e:?}"),
                )
            })?;
        }
        Ok(())
    }
}
