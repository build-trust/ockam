use crate::{fmt_log, CommandGlobalOpts, Terminal, TerminalStream};
use console::Term;
use indicatif::ProgressBar;
use ockam_api::Notification;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;
use tracing::info;

const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(100);
const REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY: Duration = Duration::from_millis(1_000);

pub struct ProgressDisplayHandle {
    stop: Arc<Mutex<bool>>,
}

impl Drop for ProgressDisplayHandle {
    fn drop(&mut self) {
        let mut stop = self.stop.lock().unwrap();
        *stop = true;
    }
}

/// This struct displays notifications coming from the CliState when commands are executed
#[derive(Debug)]
pub struct ProgressDisplay {
    notifications: Receiver<Notification>,
    /// List of all received notifications
    received: Vec<Notification>,
    /// If there is a progress bar, it is used to display messages as they arrive with a spinner
    /// and all the notifications are also displayed at the end with the terminal
    progress_bar: Option<ProgressBar>,
    /// User terminal
    terminal: Terminal<TerminalStream<Term>>,
    /// Flag to determine if the progress display should stop
    stop: Arc<Mutex<bool>>,
}

impl ProgressDisplay {
    /// Create a new NotificationsProgress without progress bar.
    /// The notifications are printed as they arrive and stay on screen
    pub fn start(opts: &CommandGlobalOpts) -> ProgressDisplayHandle {
        let stop = Arc::new(Mutex::new(false));
        let _self = ProgressDisplay {
            notifications: opts.state.subscribe(),
            received: vec![],
            terminal: opts.terminal.clone(),
            progress_bar: None,
            stop: stop.clone(),
        };
        _self.run();
        ProgressDisplayHandle { stop }
    }
}

impl ProgressDisplay {
    /// Start displaying the progress of a given action.
    /// When that action changes the values of the can_stop mutex, then the display stops.
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
            self.received.iter().for_each(|msg| {
                let _ = self.terminal.write_line(fmt_log!("{}", msg));
            });
            progress_bar.finish_and_clear();
        }

        // Additionally log all the notifications for later debugging if necessary
        self.received.iter().for_each(|msg| {
            info!(msg);
        });
    }
}
