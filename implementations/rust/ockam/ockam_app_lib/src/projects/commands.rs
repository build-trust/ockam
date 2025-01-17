use miette::IntoDiagnostic;
use ockam::Context;
use std::collections::BTreeMap;
use std::time::Duration;
use tracing::{debug, info, trace};

use ockam_api::authenticator::enrollment_tokens::TokenIssuer;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;

use crate::projects::error::Error::ListingFailed;
use crate::state::{AppState, StateKind};

use super::error::{Error, Result};

// Store the user's admin projects
impl AppState {
    pub(crate) async fn create_enrollment_ticket(
        &self,
        ctx: &Context,
        project_id: &str,
        invitation_email: &EmailAddress,
    ) -> Result<EnrollmentTicket> {
        debug!(?project_id, "Creating an enrollment ticket");
        let projects = self.projects();
        let projects_guard = projects.read().await;
        let project = projects_guard
            .iter()
            .find(|p| p.project_id() == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?
            .clone();
        let authority_node = self
            .authority_node(ctx, &project, None)
            .await
            .into_diagnostic()?;
        let otc = authority_node
            .create_token(
                &self.context(),
                BTreeMap::from([("invitation_email".to_string(), invitation_email.to_string())]),
                Some(Duration::from_secs(60 * 60 * 24 * 14)),
                None,
            )
            .await?;
        let ticket = EnrollmentTicket::new_from_project(otc, project.model())
            .await
            .into_diagnostic()?;
        Ok(ticket)
    }

    pub(crate) async fn refresh_projects(&self) -> Result<()> {
        info!("Refreshing projects");
        if !self.is_enrolled().await.unwrap_or(false) {
            return Ok(());
        }

        let node_manager = self.node_manager().await;
        let projects = node_manager
            .get_admin_projects(&self.context())
            .await
            .map_err(|e| ListingFailed(e.to_string()))?;
        debug!("Projects fetched");
        trace!(?projects);

        *self.projects().write().await = projects;
        self.mark_as_loaded(StateKind::Projects);
        self.publish_state().await;
        Ok(())
    }
}
