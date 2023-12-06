use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::project::{OrchestratorVersionInfo, Project, Projects};
use crate::nodes::InMemoryNode;

#[async_trait]
impl Projects for InMemoryNode {
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

    async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project> {
        let controller = self.create_controller().await?;
        let project = controller.get_project(ctx, project_id).await?;
        self.cli_state.store_project(project.clone()).await?;
        Ok(project)
    }

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

    async fn get_project_by_name(
        &self,
        ctx: &Context,
        project_name: &str,
    ) -> miette::Result<Project> {
        let project_id = self.cli_state.get_project_by_name(project_name).await?.id();
        self.get_project(ctx, &project_id).await
    }

    async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()> {
        let controller = self.create_controller().await?;
        controller.delete_project(ctx, space_id, project_id).await?;
        Ok(self.cli_state.delete_project(project_id).await?)
    }

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

    async fn get_admin_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>> {
        let projects = self.create_controller().await?.list_projects(ctx).await?;
        let user = self.cli_state.get_default_user().await?;
        for project in &projects {
            let mut project = project.clone();
            if !project.has_admin_with_email(&user.email) {
                project.name = project.id.clone();
            }
            self.cli_state.store_project(project).await?
        }
        Ok(projects
            .into_iter()
            .filter(|p| p.has_admin_with_email(&user.email))
            .collect::<Vec<_>>())
    }

    /// Wait until the operation associated with the project creation is complete
    /// At this stage the project node must be up and running
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
