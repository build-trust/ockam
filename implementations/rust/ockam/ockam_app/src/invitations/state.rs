use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::async_runtime::RwLock;

use ockam_api::cloud::share::{
    InvitationList, InvitationWithAccess, ReceivedInvitation, SentInvitation,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InvitationState {
    #[serde(default)]
    pub(crate) sent: Vec<SentInvitation>,
    #[serde(default)]
    pub(crate) received: Vec<ReceivedInvitation>,
    #[serde(default)]
    pub(crate) accepted: Vec<InvitationWithAccess>,
}

impl From<InvitationList> for InvitationState {
    fn from(val: InvitationList) -> Self {
        let InvitationList {
            sent,
            received,
            accepted,
        } = val;
        Self {
            sent: sent.unwrap_or_default(),
            received: received.unwrap_or_default(),
            accepted: accepted.unwrap_or_default(),
        }
    }
}

pub(crate) type SyncState = Arc<RwLock<InvitationState>>;
