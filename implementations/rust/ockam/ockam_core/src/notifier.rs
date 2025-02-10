//! This module provides a way to send high-level notifications meant to be
//! consumed by a user interface, such as the Ockam Command.

use once_cell::sync::Lazy;
use tokio::sync::broadcast::{channel, Receiver, Sender};

/// Maximum number of notifications present in the channel
const NOTIFICATIONS_CHANNEL_CAPACITY: usize = 32;

/// Global notifier channel to send notifications.
/// We use a broadcast channel to allow multiple consumers to receive notifications.
static NOTIFIER: Lazy<Sender<Notification>> = Lazy::new(|| {
    let (tx, _) = channel::<Notification>(NOTIFICATIONS_CHANNEL_CAPACITY);
    tx
});

/// Get a sender to send notifications
pub fn sender() -> Sender<Notification> {
    NOTIFIER.clone()
}

/// Get a receiver to receive notifications
pub fn receiver() -> Receiver<Notification> {
    NOTIFIER.subscribe()
}

/// Send a simple message
pub fn notify(message: impl Into<String>) {
    if let Err(err) = NOTIFIER.send(Notification::message(message)) {
        warn!(%err, "couldn't send notification");
    }
}

/// Send a message that will be attached to a spinner
pub fn notify_with_spinner(message: impl Into<String>) {
    if let Err(err) = NOTIFIER.send(Notification::progress(message)) {
        warn!(%err, "couldn't to send notification");
    }
}

/// Finish a previously set spinner with a message
pub fn notify_end_spinner(message: impl Into<Option<String>>) {
    if let Err(err) = NOTIFIER.send(Notification::progress_finish(message)) {
        warn!(%err, "couldn't to send notification");
    }
}

/// Finish a previously set spinner and clears it
pub fn notify_end_spinner_and_clear() {
    if let Err(err) = NOTIFIER.send(Notification::ProgressFinishAndClear()) {
        warn!(%err, "failed to send notification");
    }
}

/// Notification types that can be sent
#[derive(Debug, Clone, PartialEq)]
pub enum Notification {
    /// A simple message
    Message(String),
    /// A message that will be attached to a spinner
    Progress(String),
    /// Finish a previously set spinner with a message
    ProgressFinishWithMessage(String),
    /// Finish a previously set spinner and clears it
    ProgressFinishAndClear(),
}

impl Notification {
    /// Get the contents of the notification
    pub fn contents(&self) -> Option<&str> {
        match self {
            Notification::Message(contents) => Some(contents),
            Notification::Progress(contents) => Some(contents),
            Notification::ProgressFinishWithMessage(contents) => Some(contents),
            Notification::ProgressFinishAndClear() => None,
        }
    }

    /// Create a message notification
    pub fn message(contents: impl Into<String>) -> Self {
        Self::Message(contents.into())
    }

    /// Create a progress notification
    pub fn progress(contents: impl Into<String>) -> Self {
        Self::Progress(contents.into())
    }

    /// Create a progress finish notification
    pub fn progress_finish(contents: impl Into<Option<String>>) -> Self {
        match contents.into() {
            Some(contents) => Self::ProgressFinishWithMessage(contents),
            None => Self::ProgressFinishAndClear(),
        }
    }
}
