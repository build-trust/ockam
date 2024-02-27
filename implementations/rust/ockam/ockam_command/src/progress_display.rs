use crate::{fmt_log, CommandGlobalOpts, Terminal, TerminalStream};
use console::Term;
use indicatif::ProgressBar;
use ockam_api::Notification;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tracing::info;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(100);
const REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY: Duration = Duration::from_millis(1_000);

/// This struct displays notifications coming from the CliState when commands are executed
#[derive(Debug)]
pub struct ProgressDisplay {
    /// Receive notifications from the CliState
    notifications: Receiver<Notification>,
    /// List of all received notifications
    received: Vec<Notification>,
    /// If there is a progress bar, it is used to display messages as they arrive with a spinner
    /// and all the notifications are also displayed at the end with the terminal
    progress_bar: Option<ProgressBar>,
    /// User terminal
    terminal: Terminal<TerminalStream<Term>>,
}

impl ProgressDisplay {
    /// Create a new NotificationsProgress without progress bar.
    /// The notifications are printed as they arrive and stay on screen
    pub fn new(opts: &CommandGlobalOpts) -> ProgressDisplay {
        ProgressDisplay {
            notifications: opts.state.subscribe(),
            received: vec![],
            terminal: opts.terminal.clone(),
            progress_bar: None,
        }
    }

    /// Create a new NotificationsProgress with a progress bar.
    /// The notifications are displayed by the progress bar, one after the other,
    /// and can be finally displayed all at once by using the `finalize` method
    #[allow(unused)]
    pub fn new_with_progress_bar(opts: &CommandGlobalOpts) -> ProgressDisplay {
        ProgressDisplay {
            notifications: opts.state.subscribe(),
            received: vec![],
            terminal: opts.terminal.clone(),
            progress_bar: opts.terminal.progress_spinner(),
        }
    }
}

impl ProgressDisplay {
    /// Start displaying the progress of a given action.
    /// When that action changes the values of the can_stop mutex, then the display stops.
    pub async fn start(&mut self, can_stop: Arc<Mutex<bool>>) -> miette::Result<()> {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(REPORTING_CHANNEL_POLL_DELAY) => {
                    if *can_stop.lock().await {
                        self.stop_progress_bar();
                        break;
                    }
                }
                notification = self.notifications.recv() => {
                    match notification {
                        Ok(notification) => {
                            // If the progress bar is available, display the message and save it
                            // for later display
                            match self.progress_bar.as_ref() {
                                Some(progress_bar) => {
                                   self.received.push(notification.clone());
                                   // Fabricate a delay for a better UX, so the user has a chance to read the message.
                                   progress_bar.set_message(notification);
                                    let _ = tokio::time::sleep(REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY)
                                    .await;
                                },
                                None => {
                                   let _ = self.terminal.write_line(fmt_log!("{}", notification));
                               }
                            };
                        }
                        // Unknown problem with channel.
                        _ => {
                            self.stop_progress_bar();
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Finalize the notifications by displaying/logging them
    pub fn finalize(&self) {
        // If a progress bar was, then display the received notifications to user as a summary of
        // what was done. If there was no progress bar, then all the notifications have already been
        // printed out on the terminal
        if self.progress_bar.is_some() {
            self.received.iter().for_each(|msg| {
                let _ = self.terminal.write_line(fmt_log!("{}", msg));
            });
        }

        // Additionally log all the notifications for later debugging if necessary
        self.received.iter().for_each(|msg| {
            info!(msg);
        });
    }

    /// Stop the progress bar if any
    fn stop_progress_bar(&self) {
        if let Some(progress_bar) = self.progress_bar.as_ref() {
            progress_bar.finish_and_clear()
        }
    }
}
