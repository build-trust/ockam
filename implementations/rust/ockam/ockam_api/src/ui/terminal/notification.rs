use crate::terminal::{Terminal, TerminalWriter};
use crate::{fmt_log, CliState};
use indicatif::ProgressBar;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::select;

use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;

use tracing::info;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(100);
const REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY: Duration = Duration::from_millis(1_000);

pub type Notification = String;

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
    /// List of all received notifications
    notifications: Vec<Notification>,
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
            rx: cli_state.subscribe(),
            notifications: vec![],
            terminal: terminal.clone(),
            progress_bar: None,
            stop: stop.clone(),
        };
        _self.run();
        NotificationHandle { stop }
    }
}

impl<T: TerminalWriter + Debug + Send + 'static> NotificationHandler<T> {
    pub fn run(mut self) {
        tokio::spawn(async move {
            loop {
                select! {
                    _ = sleep(REPORTING_CHANNEL_POLL_DELAY) => {
                        if *self.stop.lock().unwrap() {
                            self.finalize();
                            break;
                        }
                    }
                    notification = self.rx.recv() => {
                        match notification {
                            Ok(notification) => {
                                // If the progress bar is available, display the message and save it
                                // for later display
                                match self.progress_bar.as_ref() {
                                    Some(progress_bar) => {
                                        self.notifications.push(notification.clone());
                                        // Fabricate a delay for a better UX, so the user has a chance to read the message.
                                        progress_bar.set_message(notification);
                                        let _ = sleep(REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY).await;
                                    },
                                    None => {
                                        let _ = self.terminal.write_line(fmt_log!("{}", notification));
                                    }
                                };
                            }
                            // Unknown problem with the channel.
                            _ => {
                                self.finalize();
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Finalize the notifications by displaying/logging them
    fn finalize(&self) {
        // If a progress bar was, then display the received notifications to user as a summary of
        // what was done. If there was no progress bar, then all the notifications have already been
        // printed out on the terminal
        if let Some(progress_bar) = &self.progress_bar {
            self.notifications.iter().for_each(|msg| {
                let _ = self.terminal.write_line(fmt_log!("{}", msg));
            });
            progress_bar.finish_and_clear();
        }

        // Additionally log all the notifications for later debugging if necessary
        self.notifications.iter().for_each(|msg| {
            info!(msg);
        });
    }
}
