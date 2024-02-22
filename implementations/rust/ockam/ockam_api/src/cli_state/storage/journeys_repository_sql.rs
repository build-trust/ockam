use chrono::{DateTime, Utc};
use sqlx::*;

use crate::journeys::{Journey, ProjectJourney};
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
        let query = query("INSERT OR REPLACE INTO project_journey VALUES (?, ?, ?, ?)")
            .bind(project_journey.project_id().to_sql())
            .bind(project_journey.opentelemetry_context().to_string().to_sql())
            .bind(project_journey.start().to_sql())
            .bind(
                project_journey
                    .previous_opentelemetry_context()
                    .map(|c| c.to_string().to_sql()),
            );
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_project_journey(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<ProjectJourney>> {
        let query = query_as(
            "\
        SELECT project_id, opentelemetry_context, start_datetime, previous_opentelemetry_context \
        FROM project_journey \
        WHERE  project_id = ? AND start_datetime <= ? \
        ORDER BY start_datetime DESC \
        LIMIT 1 OFFSET 0",
        )
        .bind(project_id.to_sql())
        .bind(now.to_sql());
        let row: Option<ProjectJourneyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.project_journey()).transpose()?)
    }

    async fn delete_project_journeys(&self, project_id: &str) -> Result<()> {
        let query =
            query("DELETE FROM project_journey where project_id = ?").bind(project_id.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn store_host_journey(&self, host_journey: Journey) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO host_journey VALUES (?, ?, ?)")
            .bind(host_journey.opentelemetry_context().to_string().to_sql())
            .bind(host_journey.start().to_sql())
            .bind(
                host_journey
                    .previous_opentelemetry_context()
                    .map(|c| c.to_string().to_sql()),
            );
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_host_journey(&self, now: DateTime<Utc>) -> Result<Option<Journey>> {
        let query = query_as(
            "\
        SELECT opentelemetry_context, start_datetime, previous_opentelemetry_context \
        FROM host_journey \
        WHERE start_datetime <= ? \
        ORDER BY start_datetime DESC \
        LIMIT 1 OFFSET 0",
        )
        .bind(now.to_sql());
        let row: Option<HostJourneyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.host_journey()).transpose()?)
    }
}

//  Database serialization / deserialization

/// Low-level representation of a row in the project journey table
#[derive(sqlx::FromRow)]
struct ProjectJourneyRow {
    project_id: String,
    opentelemetry_context: String,
    start_datetime: String,
    previous_opentelemetry_context: Option<String>,
}

impl ProjectJourneyRow {
    fn project_journey(&self) -> Result<ProjectJourney> {
        Ok(ProjectJourney::new(
            self.project_id.as_str(),
            self.opentelemetry_context()?,
            self.previous_opentelemetry_context()?,
            self.start()?,
        ))
    }

    fn opentelemetry_context(&self) -> Result<OpenTelemetryContext> {
        self.opentelemetry_context.clone().try_into()
    }

    fn previous_opentelemetry_context(&self) -> Result<Option<OpenTelemetryContext>> {
        self.previous_opentelemetry_context
            .clone()
            .map(|c| c.try_into())
            .transpose()
    }

    fn start(&self) -> Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339(&self.start_datetime)
            .map_err(|e| {
                ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
            })?
            .into())
    }
}

/// Low-level representation of a row in the host journey table
#[derive(sqlx::FromRow)]
struct HostJourneyRow {
    opentelemetry_context: String,
    start_datetime: String,
    previous_opentelemetry_context: Option<String>,
}

impl HostJourneyRow {
    fn host_journey(&self) -> Result<Journey> {
        Ok(Journey::new(
            self.opentelemetry_context()?,
            self.previous_opentelemetry_context()?,
            self.start()?,
        ))
    }

    fn opentelemetry_context(&self) -> Result<OpenTelemetryContext> {
        self.opentelemetry_context.clone().try_into()
    }

    fn previous_opentelemetry_context(&self) -> Result<Option<OpenTelemetryContext>> {
        self.previous_opentelemetry_context
            .clone()
            .map(|c| c.try_into())
            .transpose()
    }

    fn start(&self) -> Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339(&self.start_datetime)
            .map_err(|e| {
                ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
            })?
            .into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ops::{Add, Sub};
    use std::str::FromStr;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // the repository is initially empty
        let actual = repository.get_host_journey(Utc::now()).await?;
        assert_eq!(actual, None);

        // create and store a host journey
        let opentelemetry_context = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-01\",\"tracestate\":\"\"}").unwrap();
        let host_journey = Journey::new(opentelemetry_context.clone(), None, Utc::now());
        repository.store_host_journey(host_journey.clone()).await?;
        let actual = repository.get_host_journey(Utc::now()).await?;
        assert_eq!(actual, Some(host_journey));

        // create and store a project journey
        let project_journey =
            ProjectJourney::new("project_id", opentelemetry_context, None, Utc::now());
        repository
            .store_project_journey(project_journey.clone())
            .await?;
        let actual = repository
            .get_project_journey("project_id", Utc::now())
            .await?;
        assert_eq!(actual, Some(project_journey));

        // delete a project journey
        repository.delete_project_journeys("project_id").await?;
        let actual = repository
            .get_project_journey("project_id", Utc::now())
            .await?;
        assert_eq!(actual, None);
        Ok(())
    }

    /// This test checks that we can store host journeys with a previous / next relationship
    #[tokio::test]
    async fn test_several_host_journeys() -> Result<()> {
        let repository = create_repository().await?;

        // create and store a the first host journey
        let opentelemetry_context1 = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-01\",\"tracestate\":\"\"}").unwrap();
        let start1 = Utc::now();
        let host_journey1 = Journey::new(opentelemetry_context1.clone(), None, start1);
        repository.store_host_journey(host_journey1.clone()).await?;

        // retrieve the journey based on the time
        //   before the journey 1 start -> None
        //   equal or after the journey 1 start -> Some(journey1)
        let actual = repository
            .get_host_journey(start1.sub(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, None);

        let actual = repository.get_host_journey(start1).await?;
        assert_eq!(actual, Some(host_journey1.clone()));

        let actual = repository
            .get_host_journey(start1.add(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(host_journey1.clone()));

        // Create the next journey
        let opentelemetry_context2 = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-02\",\"tracestate\":\"\"}").unwrap();
        let start2 = start1.add(Duration::from_secs(1000));
        let host_journey2 = Journey::new(
            opentelemetry_context2.clone(),
            Some(opentelemetry_context1),
            start2,
        );
        repository.store_host_journey(host_journey2.clone()).await?;

        // retrieve the journey based on the time
        //   right before the journey 2 start -> Some(journey1)
        //   equal or after the journey 2 start -> Some(journey2)
        let actual = repository
            .get_host_journey(start2.sub(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(host_journey1.clone()));

        let actual = repository.get_host_journey(start2).await?;
        assert_eq!(actual, Some(host_journey2.clone()));
        assert_eq!(
            host_journey2.previous_opentelemetry_context(),
            Some(host_journey1.opentelemetry_context())
        );

        let actual = repository
            .get_host_journey(start2.add(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(host_journey2));

        Ok(())
    }

    /// This test checks that we can store project journeys with a previous / next relationship
    #[tokio::test]
    async fn test_several_project_journeys() -> Result<()> {
        let repository = create_repository().await?;

        // create and store a the first host journey
        let opentelemetry_context1 = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-01\",\"tracestate\":\"\"}").unwrap();
        let start1 = Utc::now();
        let project_journey1 =
            ProjectJourney::new("project_id", opentelemetry_context1.clone(), None, start1);
        repository
            .store_project_journey(project_journey1.clone())
            .await?;

        // retrieve the journey based on the time
        //   before the journey 1 start -> None
        //   equal or after the journey 1 start -> Some(journey1)
        let actual = repository
            .get_project_journey("project_id", start1.sub(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, None);

        let actual = repository.get_project_journey("project_id", start1).await?;
        assert_eq!(actual, Some(project_journey1.clone()));

        let actual = repository
            .get_project_journey("project_id", start1.add(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(project_journey1.clone()));

        // Create the next journey
        let opentelemetry_context2 = OpenTelemetryContext::from_str("{\"traceparent\":\"00-b9ce70eaad5a86ef6b9fa4db00589e86-8e2d99c5e5ed66e4-02\",\"tracestate\":\"\"}").unwrap();
        let start2 = start1.add(Duration::from_secs(1000));
        let project_journey2 = ProjectJourney::new(
            "project_id",
            opentelemetry_context2.clone(),
            Some(opentelemetry_context1),
            start2,
        );
        repository
            .store_project_journey(project_journey2.clone())
            .await?;

        // retrieve the journey based on the time
        //   right before the journey 2 start -> Some(journey1)
        //   equal or after the journey 2 start -> Some(journey2)
        let actual = repository
            .get_project_journey("project_id", start2.sub(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(project_journey1.clone()));

        let actual = repository.get_project_journey("project_id", start2).await?;
        assert_eq!(actual, Some(project_journey2.clone()));
        assert_eq!(
            project_journey2.previous_opentelemetry_context(),
            Some(project_journey1.opentelemetry_context())
        );

        let actual = repository
            .get_project_journey("project_id", start2.add(Duration::from_secs(3)))
            .await?;
        assert_eq!(actual, Some(project_journey2));

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn JourneysRepository>> {
        Ok(Arc::new(JourneysSqlxDatabase::create().await?))
    }
}
