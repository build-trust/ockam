use std::collections::HashMap;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use tracing::debug;

use ockam_api::cloud::share::{
    InvitationList, InvitationWithAccess, ReceivedInvitation, SentInvitation,
};

use crate::invitations::commands::InletDataFromInvitation;
use crate::{Error, Result};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InvitationState {
    #[serde(default)]
    pub(crate) sent: Vec<SentInvitation>,
    #[serde(default)]
    pub(crate) received: ReceivedInvitations,
    #[serde(default)]
    pub(crate) accepted: AcceptedInvitations,
}

impl InvitationState {
    pub fn replace_by(&mut self, list: InvitationList) {
        debug!("Updating invitations state");
        self.sent = list.sent.unwrap_or_default();
        self.received.invitations = list.received.unwrap_or_default();
        self.accepted.invitations = list.accepted.unwrap_or_default();
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ReceivedInvitations {
    pub(crate) invitations: Vec<ReceivedInvitation>,

    /// Status of accepted invitations, keyed by invitation id.
    pub(crate) status: Vec<(String, ReceivedInvitationStatus)>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum ReceivedInvitationStatus {
    Accepting,
    Accepted,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AcceptedInvitations {
    #[serde(default)]
    pub(crate) invitations: Vec<InvitationWithAccess>,

    /// Inlets for accepted invitations, keyed by invitation id.
    #[serde(default)]
    pub(crate) inlets: HashMap<String, Inlet>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Inlet {
    pub(crate) node_name: String,
    pub(crate) alias: String,
    pub(crate) socket_addr: SocketAddr,
    pub(crate) enabled: bool,
}

impl Inlet {
    pub(crate) fn new(data: InletDataFromInvitation) -> Result<Self> {
        let socket_addr = match data.socket_addr {
            Some(addr) => addr,
            None => return Err(Error::App("Socket address should be set".to_string())),
        };
        Ok(Self {
            node_name: data.local_node_name,
            alias: data.service_name,
            socket_addr,
            enabled: data.enabled,
        })
    }

    pub(crate) fn disable(&mut self) {
        self.enabled = false;
    }

    pub(crate) fn enable(&mut self) {
        self.enabled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::cloud::share::{RoleInShare, ShareScope};

    #[test]
    fn test_replace_by() {
        let mut state = InvitationState::default();
        assert!(state.sent.is_empty());
        assert!(state.received.invitations.is_empty());
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
        assert_eq!(state.received.invitations.len(), 1);
        assert_eq!(state.accepted.invitations.len(), 1);
    }
}
