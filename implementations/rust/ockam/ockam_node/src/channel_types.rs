/// Sender used to send payload messages
pub type MessageSender<T> = tokio::sync::mpsc::Sender<T>;
/// Receiver used to receive payload messages
pub type MessageReceiver<T> = tokio::sync::mpsc::Receiver<T>;

/// Create message channel
pub fn message_channel<T>() -> (MessageSender<T>, MessageReceiver<T>) {
    tokio::sync::mpsc::channel(16)
}

/// Router sender
pub type RouterSender<T> = tokio::sync::mpsc::Sender<T>;
/// Router receiver
pub type RouterReceiver<T> = tokio::sync::mpsc::Receiver<T>;

/// Create router channel
pub fn router_channel<T>() -> (RouterSender<T>, RouterReceiver<T>) {
    tokio::sync::mpsc::channel(64)
}

// TODO: Consider replacing with oneshot

/// Sender for small channels
pub type SmallSender<T> = tokio::sync::mpsc::Sender<T>;
/// Receiver for small channels
pub type SmallReceiver<T> = tokio::sync::mpsc::Receiver<T>;

/// Create small channel (size 1)
pub fn small_channel<T>() -> (SmallSender<T>, SmallReceiver<T>) {
    tokio::sync::mpsc::channel(1)
}
