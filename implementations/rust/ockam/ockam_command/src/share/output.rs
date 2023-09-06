use ockam_api::cloud::share::{ReceivedInvitation, SentInvitation};

use crate::error::Result;
use crate::output::Output;

impl Output for ReceivedInvitation {
    fn output(&self) -> Result<String> {
        Ok(format!(
            "{}\n  scope: {} target_id: {} (expires {})",
            self.id, self.scope, self.target_id, self.expires_at
        ))
    }
}

impl Output for SentInvitation {
    fn output(&self) -> Result<String> {
        Ok(format!(
            "{}\n  scope: {} target_id: {} (expires {}) for: {:?}",
            self.id, self.scope, self.target_id, self.expires_at, self.recipient_email,
        ))
    }
}
