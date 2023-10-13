use miette::{miette, IntoDiagnostic, WrapErr};
use tracing::{debug, error, info};

use crate::api::notification::rust::{Kind, Notification};
use crate::api::state::OrchestratorStatus;
use ockam_api::cli_state;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::{add_project_info_to_node_state, update_enrolled_identity, SpaceConfig};
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;

use crate::state::{AppState, NODE_NAME, PROJECT_NAME};
use crate::Result;

impl AppState {
    /// Enroll a user.
    ///
    /// This function:
    ///  - creates a default node, with a default identity if it doesn't exist
    ///  - connects to the OIDC service to authenticate the user of the Ockam application to retrieve a token
    ///  - connects to the Orchestrator with the retrieved token to create a project
    pub async fn enroll_user(&self) -> Result<()> {
        let result = self.enroll_with_token().await;

        if let Err(err) = result {
            error!(?err, "Failed to enroll user");
            self.update_orchestrator_status(OrchestratorStatus::Disconnected);
            self.publish_state().await;
            self.notify(Notification {
                kind: Kind::Error,
                title: "Failed to enroll user".to_string(),
                message: format!("{}", err),
            });
            return Err(err);
        }
        // Reset the node manager to include the project's setup, needed to create the relay.
        // This is necessary because the project data is used in the worker initialization,
        // which can't be rerun manually once the worker is started.
        self.reset_node_manager().await?;

        // Create the relay
        self.create_relay(
            self.context(),
            self.state().await,
            self.node_manager().await,
        )
        .await;

        info!("User enrolled successfully");
        Ok(())
    }

    async fn enroll_with_token(&self) -> Result<()> {
        if self.is_enrolled().await.unwrap_or_default() {
            debug!("User is already enrolled");
            return Ok(());
        }

        self.update_orchestrator_status(OrchestratorStatus::WaitingForToken);
        self.publish_state().await;

        // get an OIDC token
        let oidc_service = OidcService::default();
        let token = oidc_service.get_token_with_pkce().await?;

        // retrieve the user information
        let user_info = oidc_service.get_user_info(&token).await?;
        info!(?user_info, "User info retrieved successfully");
        let cli_state = self.state().await;
        cli_state
            .users_info
            .overwrite(&user_info.email, user_info.clone())?;

        if !user_info.email_verified {
            self.notify(Notification {
                kind: Kind::Information,
                title: "Email Verification Required".to_string(),
                message: "For security reasons, we need to confirm your email address.\
                     A verification email has been sent to you. \
                     Please review your inbox and follow the provided steps \
                     to complete the verification process"
                    .to_string(),
            })
        }

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

        let identifier = update_enrolled_identity(&cli_state, NODE_NAME)
            .await
            .into_diagnostic()?;
        info!(%identifier, "User enrolled successfully");

        self.notify(Notification {
            kind: Kind::Information,
            title: "Enrolled successfully!".to_string(),
            message: "You can now use the Ockam app".to_string(),
        });

        Ok(())
    }

    async fn retrieve_space(&self) -> Result<Space> {
        info!("retrieving the user's space");
        let controller = self.controller().await.into_diagnostic()?;

        // list the spaces that the user can access
        // and sort them by name to make sure to get the same space every time
        // if several spaces are available
        let spaces = {
            let mut spaces = controller
                .list_spaces(&self.context())
                .await
                .map_err(|e| miette!(e))?;
            spaces.sort_by(|s1, s2| s1.name.cmp(&s2.name));
            spaces
        };

        // take the first one that is available
        // otherwise create a space with a random name
        let space = match spaces.first() {
            Some(space) => space.clone(),
            None => {
                let space_name = cli_state::random_name();
                controller
                    .create_space(&self.context(), space_name, vec![])
                    .await
                    .map_err(|e| miette!(e))?
            }
        };
        self.state()
            .await
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;

        Ok(space)
    }

    async fn retrieve_project(&self, space: &Space) -> Result<Project> {
        info!("retrieving the user project");
        let email = self.user_email().await.wrap_err("User info is not valid")?;

        let controller = self.controller().await.into_diagnostic()?;
        let projects = controller
            .list_projects(&self.context())
            .await
            .map_err(|e| miette!(e))?;
        let admin_project = projects
            .iter()
            .filter(|p| p.has_admin_with_email(&email))
            .find(|p| p.name == *PROJECT_NAME);

        let project = match admin_project {
            Some(project) => project.clone(),
            None => {
                self.notify(Notification {
                    kind: Kind::Information,
                    title: "Creating a new project...".to_string(),
                    message: "This might take a few seconds".to_string(),
                });
                let ctx = &self.context();
                let project = controller
                    .create_project(ctx, space.id.to_string(), PROJECT_NAME.to_string(), vec![])
                    .await
                    .map_err(|e| miette!(e))?;
                controller.wait_until_project_is_ready(ctx, project).await?
            }
        };
        let cli_state = self.state().await;
        cli_state
            .projects
            .overwrite(&project.name, project.clone())?;
        add_project_info_to_node_state(NODE_NAME, &cli_state, None).await?;
        Ok(project)
    }
}
