use std::net::SocketAddr;

use tracing::debug;

use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::email_address::EmailAddress;
use ockam_api::cloud::share::CreateServiceInvitation;

use crate::state::{AppState, NODE_NAME};
use crate::Error;

pub(crate) mod commands;
pub(crate) mod state;

impl AppState {
    pub(crate) async fn build_args_for_create_service_invitation(
        &self,
        outlet_socket_addr: &SocketAddr,
        recipient_email: &EmailAddress,
        enrollment_ticket: EnrollmentTicket,
    ) -> crate::Result<CreateServiceInvitation> {
        debug!(%outlet_socket_addr, %recipient_email, "preparing payload to send invitation");
        let cli_state = self.state().await;
        let service = self
            .local_services()
            .await
            .into_iter()
            .find(|o| o.socket_addr == *outlet_socket_addr)
            .ok_or::<Error>(
                format!("The outlet {outlet_socket_addr} wasn't found in the App state").into(),
            )?;
        let project = cli_state.get_default_project().await?;

        Ok(CreateServiceInvitation::new(
            &cli_state,
            None,
            project.name(),
            recipient_email.clone(),
            NODE_NAME.to_string(),
            service.worker_addr.address().to_string(),
            enrollment_ticket,
            service.alias,
            service.scheme,
        )
        .await?)
    }
}
