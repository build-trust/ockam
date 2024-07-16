use crate::channel_types;
use crate::channel_types::{OneshotReceiver, OneshotSender};
use core::time::Duration;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::string::String;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// The receiving side of a callback
pub struct CallbackReceiver<T> {
    receiver: OneshotReceiver<T>,
}

impl<T> CallbackReceiver<T> {
    /// Waits for a message indefinitely
    pub async fn receive(self) -> ockam_core::Result<T> {
        self.receiver
            .await
            .map_err(|e| channel_closed(format!("receive failed: {e:?}")))
    }

    /// Waits for a message with a timeout
    pub async fn receive_timeout(self, timeout: Duration) -> ockam_core::Result<T> {
        let result = crate::compat::timeout(timeout, self.receiver).await;
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(channel_closed(format!(
                "receive, with timeout {:?}, failed: {e:?}",
                timeout
            ))),
            Err(_) => Err(Error::new(Origin::Node, Kind::Timeout, "timeout")),
        }
    }
}

fn channel_closed(e: String) -> Error {
    Error::new(Origin::Node, Kind::Cancelled, e)
}

/// The sender side of a callback
pub struct CallbackSender<T> {
    sender: OneshotSender<T>,
}

impl<T: Debug> CallbackSender<T> {
    /// Send a message to the callback
    pub fn send(self, data: T) -> ockam_core::Result<()> {
        self.sender
            .send(data)
            .map_err(|e| channel_closed(format!("sending data failed: {e:?}")))
    }
}

/// Creates a new callback
pub fn new_callback<T>() -> (CallbackReceiver<T>, CallbackSender<T>) {
    let (sender, receiver) = channel_types::oneshot_channel::<T>();
    (CallbackReceiver { receiver }, CallbackSender { sender })
}
