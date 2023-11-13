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

    async fn get_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>> {
        let projects = self.create_controller().await?.list_projects(ctx).await?;
        for project in &projects {
            self.cli_state.store_project(project.clone()).await?
        }
        Ok(projects)
    }
}
