use miette::IntoDiagnostic;
use ockam::transport::HostnamePort;
use ockam::Context;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::share::{CreateServiceInvitation, InvitationListKind, Invitations};
use ockam_core::Address;
use tracing::{debug, info, trace, warn};

use crate::api::notification::rust::Notification;
use crate::api::notification::Kind;
use crate::invitations::state::ReceivedInvitationStatus;
use crate::state::{AppState, StateKind, PROJECT_NAME};

impl AppState {
    /// Fetch received, accept and sent invitations from the orchestrator
    pub async fn refresh_invitations(&self) -> Result<(), String> {
        info!("Refreshing invitations");
        let invitations = {
            if !self.is_enrolled().await.unwrap_or(false) {
                debug!("not enrolled, skipping invitations refresh");
                return Ok(());
            }
            let controller = self.controller().await.map_err(|e| e.to_string())?;
            let invitations = controller
                .list_invitations(&self.context(), InvitationListKind::All)
                .await
                .map_err(|e| e.to_string())?;
            debug!("Invitations fetched");
            trace!(?invitations);
            invitations
        };

        let (changes, accepted_invitations) = {
            let invitations_arc = self.invitations();
            let mut guard = invitations_arc.write().await;
            let changes = guard.replace_by(invitations);
            if changes.changed {
                (changes, Some(guard.accepted.invitations.clone()))
            } else {
                (changes, None)
            }
        };

        self.mark_as_loaded(StateKind::Invitations);
        if changes.changed {
            self.load_services_from_invitations(accepted_invitations.unwrap())
                .await;
            self.publish_state().await;
            self.schedule_inlets_refresh_now();
            if changes.new_received_invitation {
                self.notify(Notification {
                    kind: Kind::Information,
                    title: "Pending invitations".to_string(),
                    message:
                        "You have pending portal inlet invitations, please accept or decline them."
                            .to_string(),
                })
            }
        }

        Ok(())
    }

    pub async fn accept_invitation(&self, id: String) -> Result<(), String> {
        self.accept_invitation_impl(id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn accept_invitation_impl(&self, id: String) -> crate::Result<()> {
        debug!(?id, "Accepting invitation");
        if !self.is_enrolled().await? {
            debug!(?id, "Not enrolled, invitation can't be accepted");
            return Ok(());
        }

        // Check if invitation exists
        {
            let invitations = self.invitations();
            let reader = invitations.read().await;
            if !reader.received.invitations.iter().any(|x| x.id == id) {
                debug!(?id, "Invitation doesn't exist, skipping...");
                return Ok(());
            }
        }

        // Update the invitation status to Accepting if it's not already being processed.
        // Otherwise, return early.
        {
            let invitations = self.invitations();
            let mut writer = invitations.write().await;
            match writer.received.status.iter_mut().find(|x| x.0 == id) {
                None => {
                    writer
                        .received
                        .status
                        .push((id.clone(), ReceivedInvitationStatus::Accepting));
                    debug!(?id, "Invitation is being processed");
                }
                Some((i, s)) => {
                    return match s {
                        ReceivedInvitationStatus::Accepting => {
                            debug!(?i, "Invitation is being processed");
                            Ok(())
                        }
                        ReceivedInvitationStatus::Accepted => {
                            debug!(?i, "Invitation was already accepted");
                            Ok(())
                        }
                        _ => {
                            debug!(?i, "Invitation is in status {s:?}, skipping...");
                            Ok(())
                        }
                    };
                }
            }
        }
        self.publish_state().await;

        let controller = self.controller().await?;
        let res = controller
            .accept_invitation(&self.context(), id.clone())
            .await?;

        debug!(?res);
        self.publish_state().await;
        info!(?id, "Invitation accepted");
        self.schedule_invitations_refresh_now();
        Ok(())
    }

    pub async fn ignore_invitation(&self, id: String) -> Result<(), String> {
        self.ignore_invitation_impl(id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn ignore_invitation_impl(&self, id: String) -> crate::Result<()> {
        debug!(?id, "Ignoring invitation");
        if !self.is_enrolled().await? {
            debug!(?id, "Not enrolled, invitation can't be ignored");
            return Ok(());
        }

        // Update the invitation status to Ignoring if it's not already being processed.
        // Otherwise, return early.
        {
            let invitations = self.invitations();
            let mut writer = invitations.write().await;
            match writer.received.status.iter_mut().find(|x| x.0 == id) {
                None => {
                    writer
                        .received
                        .status
                        .push((id.clone(), ReceivedInvitationStatus::Ignoring));
                    debug!(?id, "Invitation is being processed");
                }
                Some((i, s)) => match s {
                    ReceivedInvitationStatus::Ignoring => {
                        debug!(?i, "Invitation is being ignored");
                        return Ok(());
                    }
                    ReceivedInvitationStatus::Ignored => {
                        debug!(?i, "Invitation was already ignored");
                        return Ok(());
                    }
                    s => {
                        debug!(?i, "Invitation is in status {s:?}, ignoring...");
                    }
                },
            }
        }
        self.publish_state().await;

        let controller = self.controller().await?;
        controller
            .ignore_invitation(&self.context(), id.clone())
            .await?;

        self.publish_state().await;
        info!(?id, "Invitation ignored");
        self.schedule_invitations_refresh_now();
        Ok(())
    }

    pub async fn create_service_invitation_by_alias(
        &self,
        ctx: &Context,
        recipient_email: EmailAddress,
        outlet_worker_addr: &Address,
    ) -> Result<(), String> {
        let node_manager = self.node_manager().await;
        let outlets = node_manager.list_outlets();

        let to = outlets
            .into_iter()
            .find(|o| &o.worker_addr == outlet_worker_addr)
            .map(|o| o.to);

        if let Some(to) = to {
            self.create_service_invitation_by_socket_addr(ctx, recipient_email, to)
                .await
        } else {
            Err(format!("Cannot find service '{}'", outlet_worker_addr))
        }
    }

    pub async fn create_service_invitation_by_socket_addr(
        &self,
        ctx: &Context,
        recipient_email: EmailAddress,
        to: HostnamePort,
    ) -> Result<(), String> {
        info!(?recipient_email, ?to, "creating service invitation");

        let project_id = {
            // TODO: How might this degrade for users who have multiple spaces and projects?
            let projects = self.projects();
            let projects_guard = projects.read().await;
            projects_guard
                .iter()
                .find(|p| p.name() == PROJECT_NAME)
                .map(|p| p.project_id().to_string())
                .ok_or_else(|| "could not find default project".to_string())
        }?;

        let enrollment_ticket = self
            .create_enrollment_ticket(ctx, &project_id, &recipient_email)
            .await
            .map_err(|e| e.to_string())?;

        let invite_args = self
            .build_args_for_create_service_invitation(&to, &recipient_email, enrollment_ticket)
            .await
            .map_err(|e| e.to_string())?;

        let this = self.clone();
        tokio::spawn(async move {
            let result = this.send_invitation(invite_args).await;
            if let Err(e) = result {
                warn!(%e, "Failed to send invitation");
            }
        });
        Ok(())
    }

    async fn send_invitation(&self, invite_args: CreateServiceInvitation) -> crate::Result<()> {
        let controller = self.controller().await.into_diagnostic()?;
        let CreateServiceInvitation {
            expires_at,
            project_id,
            recipient_email,
            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,
            enrollment_ticket,
        } = invite_args;
        let res = controller
            .create_service_invitation(
                &self.context(),
                expires_at,
                project_id,
                recipient_email,
                project_identity,
                project_route,
                project_authority_identity,
                project_authority_route,
                shared_node_identity,
                shared_node_route,
                enrollment_ticket,
            )
            .await
            .map_err(|e| e.to_string())?;
        debug!(?res, "invitation sent");
        self.schedule_invitations_refresh_now();
        Ok(())
    }
}
