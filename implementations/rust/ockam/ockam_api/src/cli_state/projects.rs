use std::collections::HashMap;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::cli_state::CliState;
use crate::cloud::project::Project;

use super::Result;

impl CliState {
    pub async fn store_project(&self, project: Project) -> Result<()> {
        let repository = self.projects_repository().await?;
        repository.store_project(&project).await?;
        // If there is no previous default project set this project as the default
        let default_project = repository.get_default_project().await?;
        if default_project.is_none() {
            repository.set_default_project(&project.id).await?
        };

        // create a corresponding trust context
        self.create_trust_context(
            Some(project.name()),
            Some(project.id()),
            None,
            project.authority_identity().await.ok(),
            project.authority_access_route().ok(),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        let repository = self.projects_repository().await?;
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

    pub async fn set_default_project(&self, project_id: &str) -> Result<()> {
        self.projects_repository()
            .await?
            .set_default_project(project_id)
            .await?;
        Ok(())
    }

    pub async fn get_default_project(&self) -> Result<Project> {
        match self
            .projects_repository()
            .await?
            .get_default_project()
            .await?
        {
            Some(project) => Ok(project),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default project",
            ))?,
        }
    }

    pub async fn get_project_by_name(&self, name: &str) -> Result<Project> {
        match self
            .projects_repository()
            .await?
            .get_project_by_name(name)
            .await?
        {
            Some(project) => Ok(project),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no project named {name}"),
            ))?,
        }
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Project> {
        match self
            .projects_repository()
            .await?
            .get_project(project_id)
            .await?
        {
            Some(project) => Ok(project),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no space project with id {project_id}"),
            ))?,
        }
    }

    pub async fn get_project_by_name_or_default(
        &self,
        project_name: &Option<String>,
    ) -> Result<Project> {
        match project_name {
            Some(project_name) => self.get_project_by_name(project_name.as_str()).await,
            None => self.get_default_project().await,
        }
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        Ok(self.projects_repository().await?.get_projects().await?)
    }

    pub async fn get_projects_grouped_by_name(&self) -> Result<HashMap<String, Project>> {
        let mut projects = HashMap::new();
        for project in self.get_projects().await? {
            projects.insert(project.name.clone(), project);
        }
        Ok(projects)
    }
}
