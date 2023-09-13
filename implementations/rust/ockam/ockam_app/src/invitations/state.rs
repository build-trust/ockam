use std::collections::HashMap;
use std::net::SocketAddr;
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
    pub(crate) accepted: AcceptedInvitations,
}

impl InvitationState {
    pub fn replace_by(&mut self, list: InvitationList) {
        self.sent = list.sent.unwrap_or_default();
        self.received = list.received.unwrap_or_default();
        self.accepted.invitations = list.accepted.unwrap_or_default();
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AcceptedInvitations {
    #[serde(default)]
    pub(crate) invitations: Vec<InvitationWithAccess>,

    /// Inlets for accepted invitations, keyed by invitation id.
    #[serde(default)]
    pub(crate) inlets: HashMap<String, SocketAddr>,
}

impl AcceptedInvitations {
    pub fn zip(&self) -> Vec<(&InvitationWithAccess, Option<&SocketAddr>)> {
        self.invitations
            .iter()
            .map(|invitation| (invitation, self.inlets.get(&invitation.invitation.id)))
            .collect::<Vec<_>>()
    }
}

pub(crate) type SyncInvitationsState = Arc<RwLock<InvitationState>>;
