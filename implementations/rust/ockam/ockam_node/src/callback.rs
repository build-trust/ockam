use crate::channel_types;
use crate::channel_types::{OneshotReceiver, OneshotSender};
use core::time::Duration;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// The receiving side of a callback
pub struct CallbackReceiver<T> {
    receiver: OneshotReceiver<T>,
}

impl<T> CallbackReceiver<T> {
    /// Waits for a message indefinitely
    pub async fn receive(self) -> ockam_core::Result<T> {
        self.receiver.await.map_err(|_| channel_closed())
    }

    /// Waits for a message with a timeout
    pub async fn receive_timeout(self, timeout: Duration) -> ockam_core::Result<T> {
        let result = crate::compat::timeout(timeout, self.receiver).await;
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(_)) => Err(channel_closed()),
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
    sender: OneshotSender<T>,
}

impl<T> CallbackSender<T> {
    /// Send a message to the callback
    pub fn send(self, data: T) -> ockam_core::Result<()> {
        self.sender.send(data).map_err(|_| channel_closed())
    }
}

/// Creates a new callback
pub fn new_callback<T>() -> (CallbackReceiver<T>, CallbackSender<T>) {
    let (sender, receiver) = channel_types::oneshot_channel::<T>();
    (CallbackReceiver { receiver }, CallbackSender { sender })
}
