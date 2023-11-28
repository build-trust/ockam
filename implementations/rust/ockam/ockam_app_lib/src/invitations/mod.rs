use std::net::SocketAddr;

use tracing::{debug, warn};

use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::share::CreateServiceInvitation;

use crate::state::{AppState, NODE_NAME};
use crate::Error;

pub(crate) mod commands;
pub(crate) mod state;

impl AppState {
    pub(crate) async fn build_args_for_create_service_invitation(
        &self,
        outlet_socket_addr: &SocketAddr,
        recipient_email: &str,
        enrollment_ticket: EnrollmentTicket,
    ) -> crate::Result<CreateServiceInvitation> {
        debug!(%outlet_socket_addr, %recipient_email, "preparing payload to send invitation");
        let cli_state = self.state().await;
        let service_route = self
            .model(|m| {
                if m.tcp_outlets.is_empty() {
                    warn!("no outlets found in the App state");
                }
                m.tcp_outlets
                    .iter()
                    .find(|o| o.socket_addr == *outlet_socket_addr)
                    .map(|o| o.worker_address())
            })
            .await
            .ok_or::<Error>(
                format!("The outlet {outlet_socket_addr} wasn't found in the App state").into(),
            )??;
        let project = cli_state.get_default_project().await?;

        Ok(CreateServiceInvitation::new(
            &cli_state,
            None,
            project.name(),
            recipient_email.to_string(),
            NODE_NAME.to_string(),
            service_route.to_string(),
            enrollment_ticket,
        )
        .await?)
    }
}
