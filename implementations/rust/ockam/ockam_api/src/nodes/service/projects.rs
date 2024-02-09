use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::project::{OrchestratorVersionInfo, Project, Projects};
use crate::nodes::InMemoryNode;

#[async_trait]
impl Projects for InMemoryNode {
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
        self.cli_state.store_project(project.clone()).await?;
        Ok(project)
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project> {
        let controller = self.create_controller().await?;

        // try to refresh the project from the controller
        match controller.get_project(ctx, project_id).await {
            Ok(project) => self.cli_state.store_project(project.clone()).await?,
            Err(e) => warn!("could no get the project {project_id} from the controller: {e:?}"),
        }
        Ok(self.cli_state.get_project(project_id).await?)
    }

    #[instrument(skip_all, fields(project_name = project_name))]
    async fn get_project_by_name_or_default(
        &self,
        ctx: &Context,
        project_name: &Option<String>,
    ) -> miette::Result<Project> {
        let project_id = self
            .cli_state
            .get_project_by_name_or_default(project_name)
            .await?
            .id();
        self.get_project(ctx, &project_id).await
    }

    #[instrument(skip_all, fields(project_name = project_name))]
    async fn get_project_by_name(
        &self,
        ctx: &Context,
        project_name: &str,
    ) -> miette::Result<Project> {
        let project_id = self.cli_state.get_project_by_name(project_name).await?.id();
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
        Ok(self.cli_state.delete_project(project_id).await?)
    }

    #[instrument(skip_all, fields(project_name = project_name, space_name = space_name))]
    async fn delete_project_by_name(
        &self,
        ctx: &Context,
        space_name: &str,
        project_name: &str,
    ) -> miette::Result<()> {
        let space = self.cli_state.get_space_by_name(space_name).await?;
        let project = self.cli_state.get_project_by_name(project_name).await?;
        self.delete_project(ctx, &space.space_id(), &project.id())
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
            Ok(projects) => {
                for project in &projects {
                    info!("retrieved project {}/{}", project.name, project.id);
                    let mut project = project.clone();
                    // If the project has no admin role, the name is set to the project id
                    // to avoid collisions with other projects with the same name that
                    // belong to other spaces.
                    if !project.is_admin(&user) {
                        project.name = project.id.clone();
                    }
                    self.cli_state.store_project(project).await?
                }
            }
            Err(e) => warn!("could not get the list of projects from the controller {e:?}"),
        }

        // Return the admin projects
        Ok(self
            .cli_state
            .get_projects()
            .await?
            .into_iter()
            .filter(|p| p.is_admin(&user))
            .collect::<Vec<_>>())
    }

    /// Wait until the operation associated with the project creation is complete
    /// At this stage the project node must be up and running
    #[instrument(skip_all, fields(project_id = project.id))]
    async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        let project = self
            .create_controller()
            .await?
            .wait_until_project_creation_operation_is_complete(ctx, project)
            .await?;
        self.cli_state.store_project(project.clone()).await?;
        Ok(project)
    }

    /// Wait until the project is ready to be used
    /// At this stage the project authority node must be up and running
    #[instrument(skip_all, fields(project_id = project.id))]
    async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        let project = self
            .create_controller()
            .await?
            .wait_until_project_is_ready(ctx, project)
            .await?;
        self.cli_state.store_project(project.clone()).await?;
        Ok(project)
    }
}
