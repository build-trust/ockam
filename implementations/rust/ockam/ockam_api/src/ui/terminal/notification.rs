use crate::fmt_log;
use crate::terminal::{Terminal, TerminalWriter};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Release};
use indicatif::ProgressBar;
use ockam_core::notifier::Notification;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(20);

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
pub struct NotificationHandler<T: TerminalWriter + Send + 'static> {
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

impl<T: TerminalWriter + Send + 'static> NotificationHandler<T> {
    /// Create a new NotificationsProgress without progress bar.
    /// The notifications are printed as they arrive and stay on screen
    pub fn start(terminal: Terminal<T>) -> NotificationHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let _self = NotificationHandler {
            rx: ockam_core::notifier::receiver(),
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
                            debug!("stopping notification handler");
                            // Drain the channel
                            while let Ok(notification) = self.rx.try_recv() {
                                self.handle_notification(notification);
                            }
                            break;
                        }
                    }
                    notification = self.rx.recv() => {
                        if let Ok(notification) = notification {
                            trace!(?notification, "received notification");
                            self.handle_notification(notification);
                        }
                        // The channel was closed
                        else {
                            debug!("notification channel closed");
                            break;
                        }
                    }
                }
            }
        });
    }

    fn handle_notification(&mut self, notification: Notification) {
        match notification {
            Notification::Message(contents) => {
                let _ = self.terminal.write_line(self.process_contents(contents));
            }
            Notification::Progress(contents) => {
                if self.terminal.can_use_interactive_elements() {
                    if self.progress_bar.is_none() {
                        self.progress_bar = self.terminal.spinner();
                    }
                    if let Some(pb) = self.progress_bar.as_ref() {
                        pb.set_message(self.process_contents(contents));
                    }
                }
                // If the progress bar can't be used (non-tty), handle as a regular message
                else {
                    let _ = self.terminal.write_line(self.process_contents(contents));
                }
            }
            Notification::ProgressFinishWithMessage(contents) => {
                if let Some(pb) = self.progress_bar.take() {
                    pb.finish_with_message(self.process_contents(contents));
                }
            }
            Notification::ProgressFinishAndClear() => {
                if let Some(pb) = self.progress_bar.take() {
                    pb.finish_and_clear();
                }
            }
        }
    }

    fn process_contents(&self, contents: String) -> String {
        // if has an expected padding, return as is
        if contents.starts_with(crate::terminal::PADDING)
            || contents.starts_with(crate::terminal::ICON_PADDING)
        {
            contents
        }
        // if not, format as a log message
        else {
            fmt_log!("{}", contents)
        }
    }
}
