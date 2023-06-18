/// Sender used to send payload messages
pub type MessageSender<T> = crate::tokio::sync::mpsc::Sender<T>;
/// Receiver used to receive payload messages
pub type MessageReceiver<T> = crate::tokio::sync::mpsc::Receiver<T>;

/// Create message channel
pub fn message_channel<T>() -> (MessageSender<T>, MessageReceiver<T>) {
    crate::tokio::sync::mpsc::channel(16)
}

/// Router sender
pub type RouterSender<T> = crate::tokio::sync::mpsc::Sender<T>;
/// Router receiver
pub type RouterReceiver<T> = crate::tokio::sync::mpsc::Receiver<T>;

/// Create router channel
pub fn router_channel<T>() -> (RouterSender<T>, RouterReceiver<T>) {
    crate::tokio::sync::mpsc::channel(64)
}

// TODO: Consider replacing with oneshot

/// Sender for small channels
pub type SmallSender<T> = crate::tokio::sync::mpsc::Sender<T>;
/// Receiver for small channels
pub type SmallReceiver<T> = crate::tokio::sync::mpsc::Receiver<T>;

/// Create small channel (size 1)
pub fn small_channel<T>() -> (SmallSender<T>, SmallReceiver<T>) {
    crate::tokio::sync::mpsc::channel(1)
}

/// Sender for oneshot channels
pub type OneshotSender<T> = crate::tokio::sync::oneshot::Sender<T>;
/// Receiver for oneshot channels
pub type OneshotReceiver<T> = crate::tokio::sync::oneshot::Receiver<T>;

/// Create a oneshot channejl
pub fn oneshot_channel<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    crate::tokio::sync::oneshot::channel()
}
