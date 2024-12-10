use crate::cloud::project::models::ProjectModel;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

/// This trait supports the storage of projects as retrieved from the Controller
///
///  - in addition to the project data, we can set a project as the default project
///  - a project is identified by its id by default when getting it or setting it as the default
///
#[async_trait]
pub trait ProjectsRepository: Send + Sync + 'static {
    /// Store a project in the database
    /// If the project has already been stored and is updated then we take care of
    /// keeping it as the default project if it was before
    async fn store_project(&self, project: &ProjectModel) -> Result<()>;

    /// Return a project given its id
    async fn get_project(&self, project_id: &str) -> Result<Option<ProjectModel>>;

    /// Return a project given its name
    async fn get_project_by_name(&self, name: &str) -> Result<Option<ProjectModel>>;

    /// Return all the projects
    async fn get_projects(&self) -> Result<Vec<ProjectModel>>;

    /// Return the default project
    async fn get_default_project(&self) -> Result<Option<ProjectModel>>;

    /// Set one project as the default project
    async fn set_default_project(&self, project_id: &str) -> Result<()>;

    /// Delete a project
    /// Return true if the project could be deleted
    async fn delete_project(&self, project_id: &str) -> Result<()>;
}

#[async_trait]
impl<T: ProjectsRepository> ProjectsRepository for AutoRetry<T> {
    async fn store_project(&self, project: &ProjectModel) -> Result<()> {
        retry!(self.wrapped.store_project(project))
    }

    async fn get_project(&self, project_id: &str) -> Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_project(project_id))
    }

    async fn get_project_by_name(&self, name: &str) -> Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_project_by_name(name))
    }

    async fn get_projects(&self) -> Result<Vec<ProjectModel>> {
        retry!(self.wrapped.get_projects())
    }

    async fn get_default_project(&self) -> Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_default_project())
    }

    async fn set_default_project(&self, project_id: &str) -> Result<()> {
        retry!(self.wrapped.set_default_project(project_id))
    }

    async fn delete_project(&self, project_id: &str) -> Result<()> {
        retry!(self.wrapped.delete_project(project_id))
    }
}
