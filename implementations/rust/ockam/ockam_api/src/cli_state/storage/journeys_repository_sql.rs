use chrono::{DateTime, Utc};
use sqlx::*;
use std::time::SystemTime;

use crate::storage::journeys_repository::JourneysRepository;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, OpenTelemetryContext};
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

#[derive(Clone)]
pub struct JourneysSqlxDatabase {
    database: SqlxDatabase,
}

impl JourneysSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for spaces");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::application_in_memory("project journey").await?,
        ))
    }
}

#[async_trait]
impl JourneysRepository for JourneysSqlxDatabase {
    async fn store_project_journey(&self, project_journey: ProjectJourney) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO project_journey VALUES (?, ?, ?)")
            .bind(project_journey.project_id.to_sql())
            .bind(project_journey.opentelemetry_context.to_string().to_sql())
            .bind(project_journey.start.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_project_journey(&self, project_id: &str) -> Result<Option<ProjectJourney>> {
        let query = query_as("SELECT project_id, opentelemetry_context, start_datetime FROM project_journey where project_id = ?").bind(project_id.to_sql());
        let row: Option<ProjectJourneyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.project_journey()).transpose()?)
    }

    async fn delete_project_journey(&self, project_id: &str) -> Result<()> {
        let query =
            query("DELETE FROM project_journey where project_id = ?").bind(project_id.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn store_host_journey(&self, host_journey: HostJourney) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO host_journey VALUES (?, ?)")
            .bind(host_journey.opentelemetry_context.to_string().to_sql())
            .bind(host_journey.start.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_host_journey(&self) -> Result<Option<HostJourney>> {
        let query = query_as("SELECT opentelemetry_context, start_datetime FROM host_journey");
        let row: Option<HostJourneyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.host_journey()).transpose()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectJourney {
    project_id: String,
    opentelemetry_context: OpenTelemetryContext,
    start: DateTime<Utc>,
}

impl ProjectJourney {
    pub fn new(
        project_id: &str,
        opentelemetry_context: OpenTelemetryContext,
        start: DateTime<Utc>,
    ) -> ProjectJourney {
        ProjectJourney {
            project_id: project_id.to_string(),
            opentelemetry_context,
            start,
        }
    }

    pub fn to_host_journey(&self) -> HostJourney {
        HostJourney {
            opentelemetry_context: self.opentelemetry_context.clone(),
            start: self.start,
        }
    }

    pub fn opentelemetry_context(&self) -> OpenTelemetryContext {
        self.opentelemetry_context.clone()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn start_system_time(&self) -> SystemTime {
        SystemTime::from(self.start)
    }

    pub fn extract_context(&self) -> opentelemetry::Context {
        self.opentelemetry_context.extract()
    }

    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostJourney {
    opentelemetry_context: OpenTelemetryContext,
    start: DateTime<Utc>,
}

impl HostJourney {
    pub fn new(opentelemetry_context: OpenTelemetryContext, start: DateTime<Utc>) -> HostJourney {
        HostJourney {
            opentelemetry_context,
            start,
        }
    }

    pub fn opentelemetry_context(&self) -> OpenTelemetryContext {
        self.opentelemetry_context.clone()
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn start_system_time(&self) -> SystemTime {
        SystemTime::from(self.start)
    }

    pub fn extract_context(&self) -> opentelemetry::Context {
        self.opentelemetry_context.extract()
    }
}

//  Database serialization / deserialization

/// Low-level representation of a row in the project journey table
#[derive(sqlx::FromRow)]
struct ProjectJourneyRow {
    opentelemetry_context: String,
    start_datetime: String,
    project_id: String,
}

impl ProjectJourneyRow {
    pub(crate) fn project_journey(&self) -> Result<ProjectJourney> {
        Ok(ProjectJourney {
            opentelemetry_context: self.opentelemetry_context.clone().try_into()?,
            start: DateTime::parse_from_rfc3339(&self.start_datetime)
                .map_err(|e| {
                    ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
                })?
                .into(),
            project_id: self.project_id.clone(),
        })
    }
}

/// Low-level representation of a row in the host journey table
#[derive(sqlx::FromRow)]
struct HostJourneyRow {
    opentelemetry_context: String,
    start_datetime: String,
}

impl HostJourneyRow {
    pub(crate) fn host_journey(&self) -> Result<HostJourney> {
        Ok(HostJourney {
            opentelemetry_context: self.opentelemetry_context.clone().try_into()?,
            start: DateTime::parse_from_rfc3339(&self.start_datetime)
                .map_err(|e| {
                    ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
                })?
                .into(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // create and store a host journey
        let opentelemetry_context = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-01\",\"tracestate\":\"\"}").unwrap();
        let host_journey = HostJourney::new(opentelemetry_context.clone(), Utc::now());
        repository.store_host_journey(host_journey.clone()).await?;
        let actual = repository.get_host_journey().await?;
        assert_eq!(actual, Some(host_journey));

        // create and store a project journey
        let project_journey = ProjectJourney::new("project_id", opentelemetry_context, Utc::now());
        repository
            .store_project_journey(project_journey.clone())
            .await?;
        let actual = repository.get_project_journey("project_id").await?;
        assert_eq!(actual, Some(project_journey));

        // delete a project journey
        repository.delete_project_journey("project_id").await?;
        let actual = repository.get_project_journey("project_id").await?;
        assert_eq!(actual, None);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn JourneysRepository>> {
        Ok(Arc::new(JourneysSqlxDatabase::create().await?))
    }
}
