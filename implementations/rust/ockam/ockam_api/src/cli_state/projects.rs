use std::collections::HashMap;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::cli_state::CliState;
use crate::cloud::project::Project;

use super::Result;

impl CliState {
    #[instrument(skip_all, fields(project_id = project.id))]
    pub async fn store_project(&self, project: Project) -> Result<()> {
        let repository = self.projects_repository();
        repository.store_project(&project).await?;
        // If there is no previous default project set this project as the default
        let default_project = repository.get_default_project().await?;
        if default_project.is_none() {
            repository.set_default_project(&project.id).await?
        };

        Ok(())
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        let repository = self.projects_repository();
        // delete the project
        let project_exists = repository.get_project(project_id).await.is_ok();
        repository.delete_project(project_id).await?;

        // set another project as the default project
        if project_exists {
            let other_projects = repository.get_projects().await?;
            if let Some(other_project) = other_projects.first() {
                repository.set_default_project(&other_project.id()).await?;
            }
        }
        Ok(())
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn set_default_project(&self, project_id: &str) -> Result<()> {
        self.projects_repository()
            .set_default_project(project_id)
            .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn get_default_project(&self) -> Result<Project> {
        match self.projects_repository().get_default_project().await? {
            Some(project) => Ok(project),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default project",
            ))?,
        }
    }

    #[instrument(skip_all, fields(name = name))]
    pub async fn get_project_by_name(&self, name: &str) -> Result<Project> {
        match self.projects_repository().get_project_by_name(name).await? {
            Some(project) => Ok(project),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no project named {name}"),
            ))?,
        }
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    pub async fn get_project(&self, project_id: &str) -> Result<Project> {
        match self.projects_repository().get_project(project_id).await? {
            Some(project) => Ok(project),
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
        Ok(self.projects_repository().get_projects().await?)
    }

    #[instrument(skip_all)]
    pub async fn get_projects_grouped_by_name(&self) -> Result<HashMap<String, Project>> {
        let mut projects = HashMap::new();
        for project in self.get_projects().await? {
            projects.insert(project.name.clone(), project);
        }
        Ok(projects)
    }
}
