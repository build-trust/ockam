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

/// Router sender
pub type RouterSender<T> = sync::mpsc::Sender<T>;
/// Router receiver
pub type RouterReceiver<T> = sync::mpsc::Receiver<T>;

/// Create router channel
pub fn router_channel<T>() -> (RouterSender<T>, RouterReceiver<T>) {
    sync::mpsc::channel(64)
}

// TODO: Consider replacing with oneshot

/// Sender for small channels
pub type SmallSender<T> = sync::mpsc::Sender<T>;
/// Receiver for small channels
pub type SmallReceiver<T> = sync::mpsc::Receiver<T>;

/// Create small channel (size 1)
pub fn small_channel<T>() -> (SmallSender<T>, SmallReceiver<T>) {
    sync::mpsc::channel(1)
}

/// Sender for oneshot channels
pub type OneshotSender<T> = sync::oneshot::Sender<T>;
/// Receiver for oneshot channels
pub type OneshotReceiver<T> = sync::oneshot::Receiver<T>;

/// Create a oneshot channejl
pub fn oneshot_channel<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    sync::oneshot::channel()
}
