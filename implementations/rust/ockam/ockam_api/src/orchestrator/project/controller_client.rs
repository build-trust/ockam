use crate::orchestrator::operation::Operations;
use crate::orchestrator::project::models::CreateProject;
use crate::orchestrator::project::models::{OrchestratorVersionInfo, ProjectModel};
use crate::orchestrator::{ControllerClient, HasSecureClient, ORCHESTRATOR_AWAIT_TIMEOUT};

use super::models::AdminInfo;

use miette::{miette, IntoDiagnostic};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::project::Project;
use ockam_core::api::Request;
use ockam_node::Context;

impl ControllerClient {
    pub async fn create_project(
        &self,
        ctx: &Context,
        space_id: &str,
        name: &str,
        users: Vec<String>,
    ) -> miette::Result<ProjectModel> {
        trace!(%space_id, project_name = name, "creating project");
        let req = Request::post(format!("/v1/spaces/{space_id}/projects"))
            .body(CreateProject::new(name.to_string(), users));
        self.get_secure_client()
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .miette_success("create project")
    }

    pub async fn get_project(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<ProjectModel> {
        trace!(%project_id, "getting project");
        let req = Request::get(format!("/v0/{project_id}"));
        self.get_secure_client()
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .miette_success("get project")
    }

    pub async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()> {
        trace!(%space_id, %project_id, "deleting project");
        let req = Request::delete(format!("/v0/{space_id}/{project_id}"));
        self.get_secure_client()
            .tell(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .miette_success("delete project")
    }

    pub async fn get_orchestrator_version_info(
        &self,
        ctx: &Context,
    ) -> miette::Result<OrchestratorVersionInfo> {
        trace!("getting orchestrator version information");
        self.get_secure_client()
            .ask(ctx, "version_info", Request::get(""))
            .await
            .into_diagnostic()?
            .miette_success("get orchestrator version")
    }

    pub async fn add_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo> {
        trace!("adding new space administrator");
        let admins: Vec<AdminInfo> = self
            .get_secure_client()
            .ask(
                ctx,
                "spaces",
                Request::put(format!("/v0/{space_id}/admins/{email}")),
            )
            .await
            .into_diagnostic()?
            .miette_success("add space admins")?;
        admins
            .into_iter()
            .find(|a| a.email == email.to_string())
            .ok_or(miette!(
                "A user with email {email} was not added to space {space_id}"
            ))
    }

    pub async fn list_space_admins(
        &self,
        ctx: &Context,
        space_id: &str,
    ) -> miette::Result<Vec<AdminInfo>> {
        trace!("listing space administrators");
        self.get_secure_client()
            .ask(
                ctx,
                "spaces",
                Request::get(format!("/v0/{space_id}/admins")),
            )
            .await
            .into_diagnostic()?
            .miette_success("get space admins")
    }

    pub async fn delete_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()> {
        trace!("deleting space administrator");
        let _admins: Vec<AdminInfo> = self
            .get_secure_client()
            .ask(
                ctx,
                "spaces",
                Request::delete(format!("/v0/{space_id}/admins/{email}")),
            )
            .await
            .into_diagnostic()?
            .miette_success("delete space admins")?;
        Ok(())
    }

    pub async fn add_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo> {
        trace!("adding new project administrator");
        let admins: Vec<AdminInfo> = self
            .get_secure_client()
            .ask(
                ctx,
                "projects",
                Request::put(format!("/v0/{project_id}/admins/{email}")),
            )
            .await
            .into_diagnostic()?
            .miette_success("add project admins")?;
        admins
            .into_iter()
            .find(|a| a.email == email.to_string())
            .ok_or(miette!(
                "A user with email {email} was not added to proejct {project_id}"
            ))
    }

    pub async fn list_project_admins(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<Vec<AdminInfo>> {
        trace!("listing project administrators");
        self.get_secure_client()
            .ask(
                ctx,
                "projects",
                Request::get(format!("/v0/{project_id}/admins")),
            )
            .await
            .into_diagnostic()?
            .miette_success("get project admins")
    }

    pub async fn delete_project_admin(
        &self,
        ctx: &Context,
        project_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()> {
        trace!("deleting project administrator");
        let _admins: Vec<AdminInfo> = self
            .get_secure_client()
            .ask(
                ctx,
                "projects",
                Request::delete(format!("/v0/{project_id}/admins/{email}")),
            )
            .await
            .into_diagnostic()?
            .miette_success("delete project admins")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn list_projects(&self, ctx: &Context) -> miette::Result<Vec<ProjectModel>> {
        let req = Request::get("/v0");
        self.get_secure_client()
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .miette_success("list projects")
    }

    pub async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: &ProjectModel,
    ) -> miette::Result<ProjectModel> {
        let operation_id = match &project.operation_id {
            Some(operation_id) => operation_id,
            // if no operation id is present this means that the operation is already complete
            None => return Ok(project.clone()),
        };

        let result = self
            .wait_until_operation_is_complete(ctx, operation_id)
            .await;
        match result {
            Ok(()) => self.get_project(ctx, &project.id).await,
            Err(e) => Err(miette!("The project creation did not complete: {:?}", e)),
        }
    }

    pub async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: &ProjectModel,
    ) -> miette::Result<ProjectModel> {
        let retry_strategy = FixedInterval::from_millis(5000)
            .take((ORCHESTRATOR_AWAIT_TIMEOUT.as_millis() / 5000) as usize);
        Retry::spawn(retry_strategy.clone(), || async {
            if let Ok(project_model) = self.get_project(ctx, &project.id).await {
                let project = Project::import(project_model.clone())
                    .await
                    .into_diagnostic()?;
                if project.is_ready() {
                    Ok(project_model)
                } else {
                    debug!(
                        "the project {} is not ready yet. Retrying...",
                        project.project_id()
                    );
                    Err(miette!(
                        "The project {} is not ready. Please try again.",
                        project.project_id()
                    ))
                }
            } else {
                Err(miette!(
                    "The project {} could not be retrieved",
                    &project.id
                ))
            }
        })
        .await
    }
}
