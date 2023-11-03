use miette::IntoDiagnostic;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, trace, warn};

use ockam_api::authenticator::enrollment_tokens::TokenIssuer;
use ockam_api::cloud::project::Projects;
use ockam_api::config::lookup::{ProjectAuthority, ProjectLookup};
use ockam_api::{cli_state::StateDirTrait, cloud::project::Project, identity::EnrollmentTicket};

use super::error::{Error, Result};
use crate::projects::error::Error::{InternalFailure, ListingFailed};
use crate::state::AppState;

impl AppState {
    pub(crate) async fn create_enrollment_ticket(
        &self,
        project_id: String,
    ) -> Result<EnrollmentTicket> {
        debug!(?project_id, "Creating enrollment ticket");
        let projects = self.projects();
        let projects_guard = projects.read().await;
        let project = projects_guard
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?
            .clone();
        let project_authority = ProjectAuthority::from_project(&project)
            .await
            .into_diagnostic()?
            .ok_or(Error::ProjectInvalidState(
                "project has no authority set".to_string(),
            ))?;
        let authority_node = self
            .authority_node(
                project_authority.identity_id(),
                project_authority.address(),
                None,
            )
            .await
            .into_diagnostic()?;
        let otc = authority_node
            .create_token(
                &self.context(),
                HashMap::new(),
                Some(Duration::from_secs(60 * 60 * 24 * 14)),
                None,
            )
            .await?;
        let project_lookup = ProjectLookup::from_project(&project).await.ok();
        let trust_context = project.try_into().ok();
        Ok(EnrollmentTicket::new(otc, project_lookup, trust_context))
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
