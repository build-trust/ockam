use crate::state::AppState;
use crate::Error;
use ockam_api::cloud::email_address::EmailAddress;
use tracing::{debug, info};

impl AppState {
    /// Delete a Local Service and relative TCP outlet from the default node.
    pub async fn delete_local_service(&self, service_name: String) -> crate::Result<()> {
        debug!(%service_name, "Deleting a local service");
        self.model_mut(|m| m.delete_local_service(&service_name))
            .await?;
        self.publish_state().await;

        let node_manager = self.node_manager().await;
        match node_manager.delete_outlet(&service_name).await {
            Ok(_) => {
                info!(%service_name, "TCP outlet deleted");
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to delete TCP outlet".to_string())),
        }
    }

    pub async fn revoke_access_local_service(
        &self,
        revoking_service_name: String,
        email: Option<EmailAddress>,
    ) -> crate::Result<()> {
        let sent_invitations = self.invitations().read().await.sent.clone();

        // revoke every single invitation sent to that specific email address
        // for the specified service name
        let found_invitation_ids: Vec<String> = sent_invitations
            .into_iter()
            .filter(|invitation| {
                email
                    .as_ref()
                    .map(|email| *email == invitation.recipient_email)
                    .unwrap_or(true)
            })
            .filter_map(|invitation| {
                invitation.access_details.map(|details| {
                    (
                        invitation.id,
                        details.service_name().unwrap_or("".to_string()),
                    )
                })
            })
            .filter(|(_, service_name)| service_name == &revoking_service_name)
            .map(|(id, _)| id)
            .collect();

        for invitation_id in found_invitation_ids {
            self.ignore_invitation(invitation_id).await?;
        }

        Ok(())
    }
}
