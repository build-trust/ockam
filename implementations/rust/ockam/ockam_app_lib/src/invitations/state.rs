use serde::{Deserialize, Serialize};
use tracing::debug;

use ockam_api::orchestrator::share::{
    InvitationList, InvitationWithAccess, ReceivedInvitation, SentInvitation,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InvitationState {
    #[serde(default)]
    pub(crate) sent: Vec<SentInvitation>,
    #[serde(default)]
    pub(crate) received: ReceivedInvitations,
    #[serde(default)]
    pub(crate) accepted: AcceptedInvitations,
}

pub struct InvitationUpdateStatus {
    pub changed: bool,
    pub new_received_invitation: bool,
}

impl InvitationState {
    pub fn replace_by(&mut self, list: InvitationList) -> InvitationUpdateStatus {
        debug!("Updating invitations state");
        let mut status = InvitationUpdateStatus {
            changed: false,
            new_received_invitation: false,
        };

        let new_sent = list.sent.unwrap_or_default();
        if self.sent != new_sent {
            self.sent = new_sent;
            status.changed = true;
        }
        let new_received = list
            .received
            .unwrap_or_default()
            .into_iter()
            .filter(|i| !i.ignored)
            .filter(|i| !i.is_expired().unwrap_or(true))
            .collect::<Vec<_>>();
        if self.received.invitations != new_received {
            status.new_received_invitation = new_received
                .iter()
                .any(|new| !self.received.invitations.iter().any(|old| old.id == new.id));
            self.received.invitations = new_received;
            status.changed = true;
        }
        let new_accepted = list
            .accepted
            .unwrap_or_default()
            .into_iter()
            .filter(|i| !i.invitation.ignored)
            .collect::<Vec<_>>();
        if self.accepted.invitations != new_accepted {
            self.accepted.invitations = new_accepted;
            status.changed = true;
        }

        status
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
    Ignoring,
    Ignored,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AcceptedInvitations {
    #[serde(default)]
    pub(crate) invitations: Vec<InvitationWithAccess>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::orchestrator::share::{RoleInShare, ShareScope};

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
                recipient_email: "no_email@none".try_into().unwrap(),
                remaining_uses: 0,
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
                recipient_id: 0,
                access_details: None,
            }]),
            received: Some(vec![
                ReceivedInvitation {
                    id: "id1".to_string(),
                    expires_at: "2100-09-12T15:07:14.00".to_string(),
                    grant_role: RoleInShare::Admin,
                    owner_email: "owner@email".try_into().unwrap(),
                    scope: ShareScope::Project,
                    target_id: "target_id".to_string(),
                    ignored: false,
                },
                ReceivedInvitation {
                    id: "id2".to_string(),
                    expires_at: "2100-09-12T15:07:14.00".to_string(),
                    grant_role: RoleInShare::Admin,
                    owner_email: "owner@email".try_into().unwrap(),
                    scope: ShareScope::Project,
                    target_id: "target_id".to_string(),
                    ignored: true,
                },
            ]),
            accepted: Some(vec![
                InvitationWithAccess {
                    invitation: ReceivedInvitation {
                        id: "id1".to_string(),
                        expires_at: "expires_at".to_string(),
                        grant_role: RoleInShare::Admin,
                        owner_email: "owner@email".try_into().unwrap(),
                        scope: ShareScope::Project,
                        target_id: "target_id".to_string(),
                        ignored: false,
                    },
                    service_access_details: None,
                },
                InvitationWithAccess {
                    invitation: ReceivedInvitation {
                        id: "id2".to_string(),
                        expires_at: "expires_at".to_string(),
                        grant_role: RoleInShare::Admin,
                        owner_email: "owner@email".try_into().unwrap(),
                        scope: ShareScope::Project,
                        target_id: "target_id".to_string(),
                        ignored: true,
                    },
                    service_access_details: None,
                },
            ]),
        };
        assert!(state.replace_by(list.clone()).changed);
        assert!(!state.replace_by(list).changed);
        assert_eq!(state.sent.len(), 1);
        assert_eq!(state.received.invitations.len(), 1);
        assert_eq!(state.accepted.invitations.len(), 1);
    }
}
