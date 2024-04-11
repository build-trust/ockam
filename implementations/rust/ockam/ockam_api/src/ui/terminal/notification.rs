use crate::terminal::{Terminal, TerminalWriter};
use crate::{fmt_log, CliState};
use indicatif::ProgressBar;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::select;

use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq)]
pub enum Notification {
    Message(String),
    ProgressSet(String),
    ProgressFinish(Option<String>),
}

impl Notification {
    pub fn contents(&self) -> Option<&str> {
        match self {
            Notification::Message(contents) => Some(contents),
            Notification::ProgressSet(contents) => Some(contents),
            Notification::ProgressFinish(contents) => contents.as_deref(),
        }
    }

    pub fn message(contents: impl Into<String>) -> Self {
        Self::Message(contents.into())
    }

    pub fn progress_set(contents: impl Into<String>) -> Self {
        Self::ProgressSet(contents.into())
    }

    pub fn progress_finish(contents: impl Into<Option<String>>) -> Self {
        Self::ProgressFinish(contents.into())
    }
}

pub struct NotificationHandle {
    stop: Arc<Mutex<bool>>,
}

impl Drop for NotificationHandle {
    fn drop(&mut self) {
        let mut stop = self.stop.lock().unwrap();
        *stop = true;
    }
}

/// This struct displays notifications coming from the CliState when commands are executed
#[derive(Debug)]
pub struct NotificationHandler<T: TerminalWriter + Debug + Send + 'static> {
    /// Channel to receive notifications
    rx: Receiver<Notification>,
    /// If there is a progress bar, it is used to display messages as they arrive with a spinner
    /// and all the notifications are also displayed at the end with the terminal
    progress_bar: Option<ProgressBar>,
    /// User terminal
    terminal: Terminal<T>,
    /// Flag to determine if the progress display should stop
    stop: Arc<Mutex<bool>>,
}

impl<T: TerminalWriter + Debug + Send + 'static> NotificationHandler<T> {
    /// Create a new NotificationsProgress without progress bar.
    /// The notifications are printed as they arrive and stay on screen
    pub fn start(cli_state: &CliState, terminal: Terminal<T>) -> NotificationHandle {
        let stop = Arc::new(Mutex::new(false));
        let _self = NotificationHandler {
            rx: cli_state.subscribe_to_notifications(),
            terminal: terminal.clone(),
            progress_bar: None,
            stop: stop.clone(),
        };
        _self.run();
        NotificationHandle { stop }
    }

    pub fn run(mut self) {
        tokio::spawn(async move {
            loop {
                select! {
                    _ = sleep(REPORTING_CHANNEL_POLL_DELAY) => {
                        if *self.stop.lock().unwrap() {
                            break;
                        }
                    }
                    notification = self.rx.recv() => {
                        if let Ok(notification) = notification {
                            self.handle_notification(notification).await;
                        }
                        // The channel was closed
                        else {
                            break;
                        }
                    }
                }
            }
        });
    }

    async fn handle_notification(&mut self, notification: Notification) {
        match notification {
            Notification::Message(contents) => {
                let _ = self.terminal.write_line(contents);
            }
            Notification::ProgressSet(contents) => {
                if self.terminal.can_use_progress_spinner() {
                    if self.progress_bar.is_none() {
                        self.progress_bar = self.terminal.progress_spinner();
                    }
                    if let Some(pb) = self.progress_bar.as_ref() {
                        pb.set_message(contents);
                    }
                }
                // If the progress bar can't be used (non-tty), handle as a regular message
                else {
                    // Since progress bar messages are not formatted, apply the log formatting here
                    let _ = self.terminal.write_line(fmt_log!("{}", contents));
                }
            }
            Notification::ProgressFinish(contents) => {
                if let Some(pb) = self.progress_bar.take() {
                    if let Some(contents) = contents {
                        pb.finish_with_message(contents);
                    } else {
                        pb.finish_and_clear();
                    }
                }
            }
        }
    }
}
