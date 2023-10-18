use tracing::{debug, error, info, trace, warn};

use ockam_api::cloud::project::Projects;
use ockam_api::{cli_state::StateDirTrait, cloud::project::Project, identity::EnrollmentTicket};

use super::error::{Error, Result};
use crate::projects::error::Error::{InternalFailure, ListingFailed};
use crate::state::AppState;

// Store the user's admin projects
impl AppState {
    pub(crate) async fn create_enrollment_ticket(
        &self,
        project_id: String,
    ) -> Result<EnrollmentTicket> {
        let projects = self.projects();
        let projects_guard = projects.read().await;
        let project = projects_guard
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?;

        debug!(?project_id, "Creating enrollment ticket via CLI");
        // TODO: How might this degrade for users who have multiple spaces and projects?
        let background_node_client = self.background_node_client().await;
        let hex_encoded_ticket = background_node_client
            .projects()
            .ticket(&project.name)
            .await
            .map_err(|_| Error::EnrollmentTicketFailed)?;
        serde_json::from_slice(&hex::decode(hex_encoded_ticket).map_err(|err| {
            error!(?err, "Could not hex-decode enrollment ticket");
            Error::EnrollmentTicketDecodeFailed
        })?)
        .map_err(|err| {
            error!(?err, "Could not JSON-decode enrollment ticket");
            Error::EnrollmentTicketDecodeFailed
        })
    }

    pub(crate) async fn refresh_projects(&self) -> Result<()> {
        info!("Refreshing projects");
        if !self.is_enrolled().await.unwrap_or(false) {
            return Ok(());
        }
        let email = match self.user_email().await {
            Ok(email) => email,
            Err(_) => {
                warn!("User info is not available");
                return Ok(());
            }
        };

        let controller = self
            .controller()
            .await
            .map_err(|e| InternalFailure(e.to_string()))?;
        let projects = controller
            .list_projects(&self.context())
            .await
            .map_err(|e| ListingFailed(e.to_string()))?
            .into_iter()
            .filter(|p| p.has_admin_with_email(&email))
            .collect::<Vec<Project>>();
        debug!("Projects fetched");
        trace!(?projects);

        let cli_projects = self.state().await.projects;
        for project in &projects {
            cli_projects
                .overwrite(&project.name, project.clone())
                .map_err(|_| Error::StateSaveFailed)?;
        }

        *self.projects().write().await = projects;
        self.publish_state().await;
        Ok(())
    }
}
