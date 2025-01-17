use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::project::models::{AdminInfo, OrchestratorVersionInfo};
use crate::orchestrator::project::Project;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
pub trait ProjectsOrchestratorApi {
    async fn create_project(
        &self,
        ctx: &Context,
        space_id: &str,
        name: &str,
        users: Vec<String>,
    ) -> miette::Result<Project>;

    async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project>;

    async fn get_project_by_name(
        &self,
        ctx: &Context,
        project_name: &str,
    ) -> miette::Result<Project>;

    async fn get_project_by_name_or_default(
        &self,
        ctx: &Context,
        project_name: &Option<String>,
    ) -> miette::Result<Project>;

    async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()>;

    async fn delete_project_by_name(
        &self,
        ctx: &Context,
        space_name: &str,
        project_name: &str,
    ) -> miette::Result<()>;

    async fn get_orchestrator_version_info(
        &self,
        ctx: &Context,
    ) -> miette::Result<OrchestratorVersionInfo>;

    async fn get_admin_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>>;

    async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project>;

    async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project>;

    async fn add_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo>;

    async fn list_project_admins(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<Vec<AdminInfo>>;

    async fn delete_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()>;
}
