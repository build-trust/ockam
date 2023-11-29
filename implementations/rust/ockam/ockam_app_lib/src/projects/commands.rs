use miette::IntoDiagnostic;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, trace, warn};

use ockam_api::authenticator::enrollment_tokens::TokenIssuer;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::cloud::project::{Project, Projects};

use crate::projects::error::Error::ListingFailed;
use crate::state::{AppState, StateKind};

use super::error::{Error, Result};

// Store the user's admin projects
impl AppState {
    pub(crate) async fn create_enrollment_ticket(
        &self,
        project_id: &str,
        invitation_email: &str,
    ) -> Result<EnrollmentTicket> {
        debug!(?project_id, "Creating enrollment ticket");
        let projects = self.projects();
        let projects_guard = projects.read().await;
        let project = projects_guard
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?
            .clone();
        let authority_node = self
            .authority_node(
                &project.authority_identifier().await.into_diagnostic()?,
                &project.authority_access_route().into_diagnostic()?,
                None,
            )
            .await
            .into_diagnostic()?;
        let otc = authority_node
            .create_token(
                &self.context(),
                HashMap::from([("invitation_email", invitation_email)]),
                Some(Duration::from_secs(60 * 60 * 24 * 14)),
                None,
            )
            .await?;
        Ok(EnrollmentTicket::new(otc, Some(project)))
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

        let node_manager = self.node_manager().await;
        let projects = node_manager
            .get_projects(&self.context())
            .await
            .map_err(|e| ListingFailed(e.to_string()))?
            .into_iter()
            .filter(|p| p.has_admin_with_email(&email))
            .collect::<Vec<Project>>();
        debug!("Projects fetched");
        trace!(?projects);

        *self.projects().write().await = projects;
        self.mark_as_loaded(StateKind::Projects);
        self.publish_state().await;
        Ok(())
    }
}
