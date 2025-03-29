#[cfg(feature = "std")]
use tokio::sync;

#[cfg(not(feature = "std"))]
use crate::tokio::sync;

/// Sender used to send payload messages
pub type MessageSender<T> = sync::mpsc::Sender<T>;
/// Receiver used to receive payload messages
pub type MessageReceiver<T> = sync::mpsc::Receiver<T>;

/// Create message channel
pub fn message_channel<T>() -> (MessageSender<T>, MessageReceiver<T>) {
    sync::mpsc::channel(8)
}

/// Sender for oneshot channels
pub type OneshotSender<T> = sync::oneshot::Sender<T>;
/// Receiver for oneshot channels
pub type OneshotReceiver<T> = sync::oneshot::Receiver<T>;

/// Create a oneshot channejl
pub fn oneshot_channel<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    sync::oneshot::channel()
}
