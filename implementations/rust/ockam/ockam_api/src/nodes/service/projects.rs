use crate::nodes::InMemoryNode;
use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::project::models::{AdminInfo, OrchestratorVersionInfo};
use crate::orchestrator::project::{Project, ProjectsOrchestratorApi};
use miette::IntoDiagnostic;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
impl ProjectsOrchestratorApi for InMemoryNode {
    #[instrument(skip_all, fields(project_name = project_name, space_name = space_name))]
    async fn create_project(
        &self,
        ctx: &Context,
        space_name: &str,
        project_name: &str,
        users: Vec<String>,
    ) -> miette::Result<Project> {
        let space = self.cli_state.get_space_by_name(space_name).await?;
        let controller = self.create_controller().await?;
        let project = controller
            .create_project(ctx, &space.space_id(), project_name, users)
            .await?;
        let project = self
            .cli_state
            .projects()
            .import_and_store_project(project.clone())
            .await?;
        Ok(project)
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project> {
        let controller = self.create_controller().await?;

        // try to refresh the project from the controller
        match controller.get_project(ctx, project_id).await {
            Ok(project) => Ok(self
                .cli_state
                .projects()
                .import_and_store_project(project.clone())
                .await?),
            Err(e) => {
                warn!("could no get the project {project_id} from the controller: {e}");
                Ok(self.cli_state.projects().get_project(project_id).await?)
            }
        }
    }

    #[instrument(skip_all, fields(project_name = project_name))]
    async fn get_project_by_name_or_default(
        &self,
        ctx: &Context,
        project_name: &Option<String>,
    ) -> miette::Result<Project> {
        let project_id = self
            .cli_state
            .projects()
            .get_project_by_name_or_default(project_name)
            .await?
            .project_id()
            .to_string();
        self.get_project(ctx, &project_id).await
    }

    #[instrument(skip_all, fields(project_name = project_name))]
    async fn get_project_by_name(
        &self,
        ctx: &Context,
        project_name: &str,
    ) -> miette::Result<Project> {
        let project_id = self
            .cli_state
            .projects()
            .get_project_by_name(project_name)
            .await?
            .project_id()
            .to_string();
        self.get_project(ctx, &project_id).await
    }

    #[instrument(skip_all, fields(project_id = project_id, space_id = space_id))]
    async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()> {
        let controller = self.create_controller().await?;
        controller.delete_project(ctx, space_id, project_id).await?;
        self.cli_state.reset_project_journey(project_id).await?;
        Ok(self.cli_state.projects().delete_project(project_id).await?)
    }

    #[instrument(skip_all, fields(project_name = project_name, space_name = space_name))]
    async fn delete_project_by_name(
        &self,
        ctx: &Context,
        space_name: &str,
        project_name: &str,
    ) -> miette::Result<()> {
        let space = self.cli_state.get_space_by_name(space_name).await?;
        let project = self
            .cli_state
            .projects()
            .get_project_by_name(project_name)
            .await?;
        self.delete_project(ctx, &space.space_id(), project.project_id())
            .await
    }

    #[instrument(skip_all)]
    async fn get_orchestrator_version_info(
        &self,
        ctx: &Context,
    ) -> miette::Result<OrchestratorVersionInfo> {
        Ok(self
            .create_controller()
            .await?
            .get_orchestrator_version_info(ctx)
            .await?)
    }

    #[instrument(skip_all)]
    async fn get_admin_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>> {
        // If there is no user in the database, the identity used an enrollment ticket
        // but it didn't enroll to the Orchestrator. Therefore, it won't have any admin projects.
        let user = match self.cli_state.get_default_user().await {
            Ok(user) => user,
            Err(_) => return Ok(vec![]),
        };
        // Try to refresh the list of projects with the controller
        match self.create_controller().await?.list_projects(ctx).await {
            Ok(project_models) => {
                for project_model in project_models {
                    info!(
                        "retrieved project {}/{}",
                        project_model.name, project_model.id
                    );
                    let project = Project::import(project_model.clone())
                        .await
                        .into_diagnostic()?;
                    self.cli_state.projects().store_project(project).await?;
                }
            }
            Err(e) => warn!("could not get the list of projects from the controller {e:?}"),
        }

        // Return the admin projects
        Ok(self
            .cli_state
            .projects()
            .get_projects()
            .await?
            .into_iter()
            .filter(|p| p.is_admin(&user))
            .collect::<Vec<_>>())
    }

    /// Wait until the operation associated with the project creation is complete
    /// At this stage the project node must be up and running
    #[instrument(skip_all, fields(project_id = project.project_id()))]
    async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        let project = self
            .create_controller()
            .await?
            .wait_until_project_creation_operation_is_complete(ctx, project.model())
            .await?;
        let project = self
            .cli_state
            .projects()
            .import_and_store_project(project.clone())
            .await?;
        Ok(project)
    }

    /// Wait until the project is ready to be used
    /// At this stage the project authority node must be up and running
    #[instrument(skip_all, fields(project_id = project.project_id()))]
    async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        self.node_manager
            .wait_until_project_is_ready(ctx, &project)
            .await
    }

    async fn add_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo> {
        let controller = self.create_controller().await?;
        let res = controller.add_project_admin(ctx, project_id, email).await?;
        self.get_project(ctx, project_id).await?;
        Ok(res)
    }

    async fn list_project_admins(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<Vec<AdminInfo>> {
        let controller = self.create_controller().await?;
        controller.list_project_admins(ctx, project_id).await
    }

    async fn delete_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()> {
        let controller = self.create_controller().await?;
        controller
            .delete_project_admin(ctx, project_id, email)
            .await?;
        self.get_project(ctx, project_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::cli_state::projects::Projects;
    use crate::cli_state::ProjectsSqlxDatabase;
    use crate::orchestrator::project::models::ProjectModel;
    use ockam::identity::{
        identities, ChangeHistoryRepository, ChangeHistorySqlxDatabase, IdentitiesVerification,
    };
    use ockam_core::Result;
    use ockam_node::database::SqlxDatabase;
    use ockam_vault::SoftwareVaultForVerifyingSignatures;
    use quickcheck::{Arbitrary, Gen};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_project_history() -> Result<()> {
        let identities = identities().await?;

        let project_identifier = identities.identities_creation().create_identity().await?;
        let project_identity = identities.get_identity(&project_identifier).await?;
        let authority_identifier = identities.identities_creation().create_identity().await?;
        let authority_identity = identities.get_identity(&authority_identifier).await?;

        let db = SqlxDatabase::in_memory("").await?;
        let change_history_repository = Arc::new(ChangeHistorySqlxDatabase::new(db.clone()));
        let projects = Projects::new(
            Arc::new(ProjectsSqlxDatabase::new(db)),
            IdentitiesVerification::new(
                change_history_repository.clone(),
                SoftwareVaultForVerifyingSignatures::create(),
            ),
        );

        let mut g = Gen::new(100);
        let mut project_model = ProjectModel::arbitrary(&mut g);

        project_model.access_route = "".to_string();
        project_model.authority_access_route = None;

        project_model.authority_identity = Some(authority_identity.export_as_string()?);
        project_model.identity = Some(project_identifier.clone());
        project_model.project_change_history = Some(project_identity.export_as_string()?);

        assert!(change_history_repository
            .get_change_history(&project_identifier)
            .await?
            .is_none());
        assert!(change_history_repository
            .get_change_history(&authority_identifier)
            .await?
            .is_none());

        projects.import_and_store_project(project_model).await?;

        assert_eq!(
            &change_history_repository
                .get_change_history(&project_identifier)
                .await?
                .unwrap(),
            project_identity.change_history()
        );
        assert_eq!(
            &change_history_repository
                .get_change_history(&authority_identifier)
                .await?
                .unwrap(),
            authority_identity.change_history()
        );

        Ok(())
    }
}
