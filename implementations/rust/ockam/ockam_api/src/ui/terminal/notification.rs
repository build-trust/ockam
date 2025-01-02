use crate::terminal::{Terminal, TerminalWriter};
use crate::{fmt_log, CliState};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Release};
use indicatif::ProgressBar;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;

use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq)]
pub enum Notification {
    Message(String),
    Progress(String),
    ProgressFinishWithMessage(String),
    ProgressFinishAndClear(),
}

impl Notification {
    pub fn contents(&self) -> Option<&str> {
        match self {
            Notification::Message(contents) => Some(contents),
            Notification::Progress(contents) => Some(contents),
            Notification::ProgressFinishWithMessage(contents) => Some(contents),
            Notification::ProgressFinishAndClear() => None,
        }
    }

    pub fn message(contents: impl Into<String>) -> Self {
        Self::Message(contents.into())
    }

    pub fn progress(contents: impl Into<String>) -> Self {
        Self::Progress(contents.into())
    }

    pub fn progress_finish(contents: impl Into<Option<String>>) -> Self {
        match contents.into() {
            Some(contents) => Self::ProgressFinishWithMessage(contents),
            None => Self::ProgressFinishAndClear(),
        }
    }
}

pub struct NotificationHandle {
    stop: Arc<AtomicBool>,
}

impl Drop for NotificationHandle {
    fn drop(&mut self) {
        self.stop.store(true, Release);
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
    stop: Arc<AtomicBool>,
}

impl<T: TerminalWriter + Debug + Send + 'static> NotificationHandler<T> {
    /// Create a new NotificationsProgress without progress bar.
    /// The notifications are printed as they arrive and stay on screen
    pub fn start(cli_state: &CliState, terminal: Terminal<T>) -> NotificationHandle {
        let stop = Arc::new(AtomicBool::new(false));
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
                        if self.stop.load(Acquire) {
                            // Drain the channel
                            while let Ok(notification) = self.rx.try_recv() {
                                self.handle_notification(notification).await;
                            }
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
            Notification::Progress(contents) => {
                if self.terminal.can_use_progress_bar() {
                    if self.progress_bar.is_none() {
                        self.progress_bar = self.terminal.spinner();
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
            Notification::ProgressFinishWithMessage(contents) => {
                if let Some(pb) = self.progress_bar.take() {
                    pb.finish_with_message(contents);
                }
            }
            Notification::ProgressFinishAndClear() => {
                if let Some(pb) = self.progress_bar.take() {
                    pb.finish_and_clear();
                }
            }
        }
    }
}
