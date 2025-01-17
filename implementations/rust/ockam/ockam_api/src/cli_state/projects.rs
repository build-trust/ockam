use ockam::identity::{Identifier, IdentitiesVerification};
use std::collections::HashMap;
use std::sync::Arc;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_vault::SoftwareVaultForVerifyingSignatures;

use crate::cli_state::{CliState, EnrollmentFilter, ProjectsRepository};
use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::project::models::ProjectModel;
use crate::orchestrator::project::Project;
use crate::orchestrator::share::RoleInShare;

use super::Result;

pub struct Projects {
    projects_repository: Arc<dyn ProjectsRepository>,
    identities_verification: IdentitiesVerification,
}

impl Projects {
    pub fn new(
        projects_repository: Arc<dyn ProjectsRepository>,
        identities_verification: IdentitiesVerification,
    ) -> Self {
        Self {
            projects_repository,
            identities_verification,
        }
    }

    #[instrument(skip_all, fields(project_id = project_model.id))]
    pub async fn import_and_store_project(&self, project_model: ProjectModel) -> Result<Project> {
        let project = Project::import(project_model.clone()).await?;
        self.store_project(project).await
    }

    #[instrument(skip_all, fields(project_id = project.project_id()))]
    pub async fn store_project(&self, project: Project) -> Result<Project> {
        if let Some(project_identity) = project.project_identity() {
            self.identities_verification
                .update_identity_ignore_older(project_identity)
                .await?;
        }

        if let Some(authority_identity) = project.authority_identity() {
            self.identities_verification
                .update_identity_ignore_older(authority_identity)
                .await?;
        }

        self.store_project_model(project.model()).await?;
        Ok(project)
    }

    #[instrument(skip_all, fields(project_id = project.id))]
    pub async fn store_project_model(&self, project: &ProjectModel) -> Result<()> {
        self.projects_repository.store_project(project).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        self.projects_repository.delete_project(project_id).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn set_default_project(&self, project_id: &str) -> Result<()> {
        self.projects_repository
            .set_default_project(project_id)
            .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn get_default_project(&self) -> Result<Project> {
        match self.projects_repository.get_default_project().await? {
            Some(project) => Ok(Project::import(project).await?),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default project",
            ))?,
        }
    }

    #[instrument(skip_all, fields(name = name))]
    pub async fn get_project_by_name(&self, name: &str) -> Result<Project> {
        match self.projects_repository.get_project_by_name(name).await? {
            Some(project) => Ok(Project::import(project).await?),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no project named {name}"),
            ))?,
        }
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn get_project(&self, project_id: &str) -> Result<Project> {
        match self.projects_repository.get_project(project_id).await? {
            Some(project) => Ok(Project::import(project).await?),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no space project with id {project_id}"),
            ))?,
        }
    }

    #[instrument(skip_all, fields(project_name = project_name.clone()))]
    pub async fn get_project_by_name_or_default(
        &self,
        project_name: &Option<String>,
    ) -> Result<Project> {
        match project_name {
            Some(project_name) => self.get_project_by_name(project_name.as_str()).await,
            None => self.get_default_project().await,
        }
    }

    #[instrument(skip_all)]
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        let project_models = self.projects_repository.get_projects().await?;

        let mut projects = Vec::with_capacity(project_models.len());
        for project_model in project_models {
            let project = Project::import(project_model).await?;
            projects.push(project);
        }

        Ok(projects)
    }

    #[instrument(skip_all)]
    pub async fn get_projects_grouped_by_name(&self) -> Result<HashMap<String, Project>> {
        let mut projects = HashMap::new();
        for project in self.get_projects().await? {
            projects.insert(project.name().to_string(), project);
        }
        Ok(projects)
    }
}

impl CliState {
    pub async fn is_project_admin(
        &self,
        caller_identifier: &Identifier,
        project: &Project,
    ) -> Result<bool> {
        let enrolled = self
            .get_identity_enrollments(EnrollmentFilter::Enrolled)
            .await?;

        let emails: Vec<EmailAddress> = enrolled
            .iter()
            .flat_map(|x| {
                if x.identifier() == caller_identifier {
                    x.status().email().cloned()
                } else {
                    None
                }
            })
            .collect();

        let is_project_admin = project
            .model()
            .user_roles
            .iter()
            .any(|u| u.role == RoleInShare::Admin && emails.contains(&u.email));

        Ok(is_project_admin)
    }

    pub fn projects(&self) -> Projects {
        let identities_verification = IdentitiesVerification::new(
            self.change_history_repository(),
            SoftwareVaultForVerifyingSignatures::create(),
        );

        Projects::new(self.projects_repository(), identities_verification)
    }
}
