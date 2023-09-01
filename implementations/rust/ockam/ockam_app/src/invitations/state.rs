use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::async_runtime::RwLock;

use ockam_api::cloud::share::{InvitationList, ReceivedInvitation, SentInvitation};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InvitationState {
    #[serde(default)]
    pub(crate) sent: Vec<SentInvitation>,
    #[serde(default)]
    pub(crate) received: Vec<ReceivedInvitation>,
}

impl InvitationState {
    pub fn replace_by(&mut self, list: InvitationList) {
        self.sent = list.sent.unwrap_or_default();
        self.received = list.received.unwrap_or_default();
    }
}

pub(crate) type SyncState = Arc<RwLock<InvitationState>>;
