use crate::CliState;
use std::time::Duration;

pub const REPORTING_CHANNEL_POLL_DELAY: Duration = Duration::from_millis(100);
pub const REPORTING_CHANNEL_MESSAGE_DISPLAY_DELAY: Duration = Duration::from_millis(1_000);
// 00: consider making this an enum w/ hints about which messages to log (all messages can be printed to stderr or spinner by default)
pub type ReportingChannelMessageType = String;
pub const REPORTING_CHANNEL_CAPACITY: usize = 16;

// Broadcast channel support.
impl CliState {
    pub fn open_channel(&mut self) -> tokio::sync::broadcast::Sender<ReportingChannelMessageType> {
        self.progress_bar_reporting_channel_sender.clone()
    }

    // 00: make the msg arg a ref
    pub fn send_over_channel(&self, msg: ReportingChannelMessageType) {
        let _ = self.progress_bar_reporting_channel_sender.send(msg);
    }
}
