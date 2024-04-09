use crate::cli_state::journeys::{Journey, ProjectJourney};
use chrono::{DateTime, Utc};
use ockam_core::async_trait;
use ockam_core::Result;

#[async_trait]
pub trait JourneysRepository: Send + Sync + 'static {
    /// Store a project journey
    async fn store_project_journey(&self, project_journey: ProjectJourney) -> Result<()>;

    /// Return the most recent project journey started after now
    async fn get_project_journey(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<ProjectJourney>>;

    /// Delete all the journeys related to a given project
    async fn delete_project_journeys(&self, project_id: &str) -> Result<()>;

    /// Store a host journey
    async fn store_host_journey(&self, host_journey: Journey) -> Result<()>;

    /// Return the most recent host journey started after now
    async fn get_host_journey(&self, now: DateTime<Utc>) -> Result<Option<Journey>>;
}
