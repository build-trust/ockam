use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::async_runtime::RwLock;

use ockam_api::cloud::share::{InvitationList, InvitationWithAccess};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteServicesState {
    #[serde(default)]
    pub(crate) services: RemoteServices,
}

impl RemoteServicesState {
    pub fn replace_by(&mut self, list: InvitationList) {
        self.services.invitations = list.accepted.unwrap_or_default();
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteServices {
    #[serde(default)]
    pub(crate) invitations: Vec<InvitationWithAccess>,

    /// Inlets for accepted invitations, keyed by invitation id.
    #[serde(default)]
    pub(crate) inlets: HashMap<String, SocketAddr>,
}

impl RemoteServices {
    pub fn zip(&self) -> Vec<(&InvitationWithAccess, Option<&SocketAddr>)> {
        self.invitations
            .iter()
            .map(|invitation| (invitation, self.inlets.get(&invitation.invitation.id)))
            .collect::<Vec<_>>()
    }
}

pub(crate) type SyncState = Arc<RwLock<RemoteServicesState>>;
