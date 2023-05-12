use crate::channel_types;
use crate::channel_types::{SmallReceiver, SmallSender};
use core::time::Duration;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// The receiving side of a callback
pub struct CallbackReceiver<T> {
    receiver: SmallReceiver<T>,
}

impl<T> CallbackReceiver<T> {
    /// Waits for a message indefinitely
    pub async fn receive(&mut self) -> ockam_core::Result<T> {
        self.receiver.recv().await.ok_or_else(channel_closed)
    }

    /// Waits for a message with a timeout
    pub async fn receive_timeout(&mut self, timeout: Duration) -> ockam_core::Result<T> {
        let result = crate::compat::timeout(timeout, self.receiver.recv()).await;
        match result {
            Ok(Some(data)) => Ok(data),
            Ok(None) => Err(channel_closed()),
            Err(_) => Err(ockam_core::Error::new(
                Origin::Node,
                Kind::Timeout,
                "timeout",
            )),
        }
    }
}

fn channel_closed() -> Error {
    Error::new(Origin::Node, Kind::Cancelled, "channel closed")
}

/// The sender side of a callback
pub struct CallbackSender<T> {
    sender: SmallSender<T>,
}

impl<T> CallbackSender<T> {
    /// Send a message to the callback
    pub async fn send(&self, data: T) -> ockam_core::Result<()> {
        self.sender.send(data).await.map_err(|_| channel_closed())
    }
}

/// Creates a new callback
pub fn new_callback<T>() -> (CallbackReceiver<T>, CallbackSender<T>) {
    let (sender, receiver) = channel_types::small_channel::<T>();
    (CallbackReceiver { receiver }, CallbackSender { sender })
}
