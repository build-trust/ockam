use miette::IntoDiagnostic;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use ockam_api::cli_state;
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::orchestrator::project::{Project, ProjectsOrchestratorApi};
use ockam_api::orchestrator::space::{Space, Spaces};

use crate::api::notification::rust::{Kind, Notification};
use crate::api::state::OrchestratorStatus;
use crate::state::{AppState, NODE_NAME, PROJECT_NAME};
use crate::Result;

enum EnrollmentOutcome {
    AlreadyEnrolled,
    PendingValidation,
    Successful,
}

impl AppState {
    /// Enroll a user.
    ///
    /// This function:
    ///  - creates a default node, with a default identity if it doesn't exist
    ///  - connects to the OIDC service to authenticate the user of the Ockam application to retrieve a token
    ///  - connects to the Orchestrator with the retrieved token to create a project
    pub async fn enroll_user(&self) -> Result<()> {
        let result = self.enroll_with_token().await;

        match result {
            Ok(outcome) => match outcome {
                EnrollmentOutcome::AlreadyEnrolled => {
                    return Ok(());
                }
                EnrollmentOutcome::PendingValidation => {
                    self.update_orchestrator_status(OrchestratorStatus::Disconnected);
                    self.publish_state().await;
                    return Ok(());
                }
                EnrollmentOutcome::Successful => {
                    // notify and keep going
                    self.notify(Notification {
                        kind: Kind::Information,
                        title: "Enrolled successfully".to_string(),
                        message: "You're ready to create your first portal.".to_string(),
                    });
                }
            },
            Err(err) => {
                error!(?err, "Failed to enroll");
                self.update_orchestrator_status(OrchestratorStatus::Disconnected);
                self.publish_state().await;
                self.notify(Notification {
                    kind: Kind::Error,
                    title: "Failed to enroll".to_string(),
                    message: format!("{}", err),
                });
                return Err(err);
            }
        }

        // Reset the node manager to include the project's setup, needed to create the relay.
        // This is necessary because the project data is used in the worker initialization,
        // which can't be rerun manually once the worker is started.
        self.reset_node_manager().await?;

        // trigger the relay connection right away
        self.schedule_relay_refresh_now();

        // avoid time gap between the enrollment and the invitations appearing in the UI
        self.schedule_invitations_refresh_now();

        // if we don't have project list and the user creates a service right away
        // it would break service sharing
        self.schedule_projects_refresh_now();

        info!("User enrolled successfully");
        Ok(())
    }

    async fn enroll_with_token(&self) -> Result<EnrollmentOutcome> {
        if self.is_enrolled().await.unwrap_or_default() {
            debug!("User is already enrolled");
            return Ok(EnrollmentOutcome::AlreadyEnrolled);
        }

        self.update_orchestrator_status(OrchestratorStatus::WaitingForToken);
        self.publish_state().await;

        // get an OIDC token
        let oidc_service = OidcService::new()?;
        let token = oidc_service.get_token_with_pkce().await?;

        // retrieve the user information
        let mut user_info = oidc_service.get_user_info(&token).await?;
        info!(?user_info, "User info retrieved successfully");

        if !user_info.email_verified {
            self.update_orchestrator_status(OrchestratorStatus::WaitingForEmailValidation);
            self.publish_state().await;

            // let's wait up to 10 minutes for the email to get validated
            let timeout_timestamp = Instant::now() + Duration::from_secs(60 * 10);
            while !user_info.email_verified {
                if Instant::now() > timeout_timestamp {
                    warn!("Timeout waiting for email validation");
                    return Ok(EnrollmentOutcome::PendingValidation);
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
                user_info = oidc_service.get_user_info(&token).await?;
            }
        }

        let cli_state = self.state().await;
        cli_state.store_user(&user_info).await?;
        cli_state.set_default_user(&user_info.email).await?;

        // enroll the current user using that token on the controller
        {
            let controller = self.controller().await.into_diagnostic()?;
            controller
                .enroll_with_oidc_token(&self.context(), token)
                .await?;
        }
        self.update_orchestrator_status(OrchestratorStatus::RetrievingSpace);
        self.publish_state().await;
        let space = self.retrieve_space().await?;

        self.update_orchestrator_status(OrchestratorStatus::RetrievingProject);
        self.publish_state().await;
        self.retrieve_project(&space).await?;

        let cli_state = self.state().await;
        let node = cli_state.get_node(NODE_NAME).await?;
        let identifier = node.identifier();
        cli_state
            .set_identifier_as_enrolled(&identifier, &user_info.email)
            .await
            .into_diagnostic()?;
        info!(%identifier, "User enrolled successfully");

        Ok(EnrollmentOutcome::Successful)
    }

    async fn retrieve_space(&self) -> Result<Space> {
        info!("retrieving the user's space");
        let node_manager = self.node_manager().await;
        let context = self.context();

        // list the spaces that the user can access
        // and sort them by name to make sure to get the same space every time
        // if several spaces are available
        let spaces = {
            let mut spaces = node_manager.get_spaces(&context).await?;
            spaces.sort_by(|s1, s2| s1.name.cmp(&s2.name));
            spaces
        };

        // take the first one that is available
        // otherwise create a space with a random name
        let space = match spaces.first() {
            Some(space) => space.clone(),
            None => {
                let space_name = cli_state::random_name();
                node_manager
                    .create_space(&self.context(), &space_name, vec![])
                    .await?
            }
        };

        Ok(space)
    }

    async fn retrieve_project(&self, space: &Space) -> Result<Project> {
        info!("retrieving the user project");
        let node_manager = self.node_manager().await;
        let projects = node_manager.get_admin_projects(&self.context()).await?;
        let main_project = projects.iter().find(|p| p.name() == PROJECT_NAME);

        let project = match main_project {
            Some(project) => project.clone(),
            None => {
                self.notify(Notification {
                    kind: Kind::Information,
                    title: "Provisioning a project".to_string(),
                    message:
                        "We're provisioning a dedicated project for you in Ockam Orchestrator. \
                        This can take up to 3 minutes."
                            .to_string(),
                });
                let ctx = &self.context();
                let project = node_manager
                    .create_project(ctx, &space.name, PROJECT_NAME, vec![])
                    .await?;
                let project = node_manager
                    .wait_until_project_creation_operation_is_complete(ctx, project)
                    .await?;
                node_manager
                    .wait_until_project_is_ready(ctx, project)
                    .await?
            }
        };
        // set the selected project as the default one
        self.state()
            .await
            .projects()
            .set_default_project(project.project_id())
            .await?;
        Ok(project)
    }
}
