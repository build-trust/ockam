use crate::{HostJourney, ProjectJourney};
use ockam_core::async_trait;
use ockam_core::Result;

#[async_trait]
pub trait JourneysRepository: Send + Sync + 'static {
    /// Store a project journey
    async fn store_project_journey(&self, project_journey: ProjectJourney) -> Result<()>;

    async fn get_project_journey(&self, project_id: &str) -> Result<Option<ProjectJourney>>;

    /// Store a host journey
    async fn store_host_journey(&self, host_journey: HostJourney) -> Result<()>;

    async fn get_host_journey(&self) -> Result<Option<HostJourney>>;
}
