use ockam_core::async_trait;
use ockam_core::Result;

use crate::cloud::project::Project;

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
    async fn store_project(&self, project: &Project) -> Result<()>;

    /// Return a project given its id
    async fn get_project(&self, project_id: &str) -> Result<Option<Project>>;

    /// Return a project given its name
    async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>>;

    /// Return all the projects
    async fn get_projects(&self) -> Result<Vec<Project>>;

    /// Return the default project
    async fn get_default_project(&self) -> Result<Option<Project>>;

    /// Set one project as the default project
    async fn set_default_project(&self, project_id: &str) -> Result<()>;

    /// Delete a project
    /// Return true if the project could be deleted
    async fn delete_project(&self, project_id: &str) -> Result<()>;
}
