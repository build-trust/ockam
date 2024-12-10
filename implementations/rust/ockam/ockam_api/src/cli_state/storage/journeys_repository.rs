use crate::cli_state::journeys::{Journey, ProjectJourney};
use chrono::{DateTime, Utc};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

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

#[async_trait]
impl<T: JourneysRepository> JourneysRepository for AutoRetry<T> {
    async fn store_project_journey(&self, project_journey: ProjectJourney) -> Result<()> {
        retry!(self.wrapped.store_project_journey(project_journey.clone()))
    }

    async fn get_project_journey(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<ProjectJourney>> {
        retry!(self.wrapped.get_project_journey(project_id, now))
    }

    async fn delete_project_journeys(&self, project_id: &str) -> Result<()> {
        retry!(self.wrapped.delete_project_journeys(project_id))
    }

    async fn store_host_journey(&self, host_journey: Journey) -> Result<()> {
        retry!(self.wrapped.store_host_journey(host_journey.clone()))
    }

    async fn get_host_journey(&self, now: DateTime<Utc>) -> Result<Option<Journey>> {
        retry!(self.wrapped.get_host_journey(now))
    }
}
