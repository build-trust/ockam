use ockam::transport::HostnamePort;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::share::CreateServiceInvitation;
use tracing::{debug, warn};

use crate::state::{AppState, NODE_NAME};
use crate::Error;

pub(crate) mod commands;
pub(crate) mod state;

impl AppState {
    pub(crate) async fn build_args_for_create_service_invitation(
        &self,
        to: &HostnamePort,
        recipient_email: &EmailAddress,
        enrollment_ticket: EnrollmentTicket,
    ) -> crate::Result<CreateServiceInvitation> {
        debug!(%to, %recipient_email, "preparing payload to send invitation");
        let cli_state = self.state().await;
        let service_route = self
            .model(|m| {
                if m.tcp_outlets.is_empty() {
                    warn!("no outlets found in the App state");
                }
                m.tcp_outlets
                    .iter()
                    .find(|o| &o.to == to)
                    .map(|o| o.worker_route())
            })
            .await
            .ok_or::<Error>(format!("The outlet {to} wasn't found in the App state").into())??;
        let project = cli_state.projects().get_default_project().await?;

        Ok(CreateServiceInvitation::new(
            &cli_state,
            None,
            project.name().to_string(),
            recipient_email.clone(),
            NODE_NAME.to_string(),
            service_route.to_string(),
            enrollment_ticket,
        )
        .await?)
    }
}
