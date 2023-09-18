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
    pub(crate) inlets: HashMap<String, TcpInlet>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TcpInlet {
    pub(crate) socket_addr: SocketAddr,
    pub(crate) enabled: bool,
}

impl TcpInlet {
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self {
            socket_addr,
            enabled: true,
        }
    }
}

pub(crate) type SyncInvitationsState = Arc<RwLock<InvitationState>>;

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::cloud::share::{RoleInShare, ShareScope};

    #[test]
    fn test_replace_by() {
        let mut state = InvitationState::default();
        assert!(state.sent.is_empty());
        assert!(state.received.is_empty());
        assert!(state.accepted.invitations.is_empty());
        let list = InvitationList {
            sent: Some(vec![SentInvitation {
                id: "id".to_string(),
                expires_at: "expires_at".to_string(),
                grant_role: RoleInShare::Admin,
                owner_id: 0,
                recipient_email: "".to_string(),
                remaining_uses: 0,
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
            }]),
            received: Some(vec![ReceivedInvitation {
                id: "id".to_string(),
                expires_at: "expires_at".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner_email".to_string(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
            }]),
            accepted: Some(vec![InvitationWithAccess {
                invitation: ReceivedInvitation {
                    id: "id".to_string(),
                    expires_at: "expires_at".to_string(),
                    grant_role: RoleInShare::Admin,
                    owner_email: "owner_email".to_string(),
                    scope: ShareScope::Project,
                    target_id: "target_id".to_string(),
                },
                service_access_details: None,
            }]),
        };
        state.replace_by(list);
        assert_eq!(state.sent.len(), 1);
        assert_eq!(state.received.len(), 1);
        assert_eq!(state.accepted.invitations.len(), 1);
    }
}
