use sqlx::any::AnyRow;
use sqlx::FromRow;
use sqlx::*;
use std::str::FromStr;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::cli_state::enrollments::IdentityEnrollment;
use crate::cli_state::EnrollmentsRepository;
use crate::orchestrator::email_address::EmailAddress;
use ockam::identity::Identifier;
use ockam::{FromSqlxError, SqlxDatabase, ToVoid};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{Boolean, Nullable};

#[derive(Clone)]
pub struct EnrollmentsSqlxDatabase {
    database: SqlxDatabase,
}

impl EnrollmentsSqlxDatabase {
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for enrollments");
        Self { database }
    }

    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn EnrollmentsRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    #[allow(unused)]
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("enrollments").await?))
    }
}

#[async_trait]
impl EnrollmentsRepository for EnrollmentsSqlxDatabase {
    async fn set_as_enrolled(&self, identifier: &Identifier, email: &EmailAddress) -> Result<()> {
        let query = query(
            r#"
                INSERT INTO identity_enrollment (identifier, enrolled_at, email)
                VALUES ($1, $2, $3)
                ON CONFLICT (identifier)
                DO UPDATE SET enrolled_at = $2, email = $3"#,
        )
        .bind(identifier)
        .bind(OffsetDateTime::now_utc().unix_timestamp())
        .bind(email);
        Ok(query.execute(&*self.database.pool).await.void()?)
    }

    async fn get_enrolled_identities(&self) -> Result<Vec<IdentityEnrollment>> {
        let query = query_as(
            r#"
            SELECT
              identity.identifier, named_identity.name, named_identity.is_default,
              identity_enrollment.enrolled_at, identity_enrollment.email
            FROM identity
            INNER JOIN identity_enrollment ON
              identity.identifier = identity_enrollment.identifier
            INNER JOIN named_identity ON
              identity.identifier = named_identity.identifier
            "#,
        )
        .bind(None as Option<i64>);
        let result: Vec<EnrollmentRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        result
            .into_iter()
            .map(|r| r.identity_enrollment())
            .collect::<Result<Vec<_>>>()
    }

    async fn get_all_identities_enrollments(&self) -> Result<Vec<IdentityEnrollment>> {
        let query = query_as(
            r#"
            SELECT
              identity.identifier, named_identity.name, named_identity.is_default,
              identity_enrollment.enrolled_at, identity_enrollment.email
            FROM identity
            LEFT JOIN identity_enrollment ON
              identity.identifier = identity_enrollment.identifier
            INNER JOIN named_identity ON
              identity.identifier = named_identity.identifier
            "#,
        );
        let result: Vec<EnrollmentRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        result
            .into_iter()
            .map(|r| r.identity_enrollment())
            .collect::<Result<Vec<_>>>()
    }

    async fn is_default_identity_enrolled(&self) -> Result<bool> {
        let query = query(
            r#"
            SELECT
              identity_enrollment.enrolled_at
            FROM identity
            INNER JOIN identity_enrollment ON
              identity.identifier = identity_enrollment.identifier
            INNER JOIN named_identity ON
              identity.identifier = named_identity.identifier
            WHERE
              named_identity.is_default = $1
            "#,
        )
        .bind(true);
        let result: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|_| true).unwrap_or(false))
    }

    async fn is_identity_enrolled(&self, name: &str) -> Result<bool> {
        let query = query(
            r#"
            SELECT
              identity_enrollment.enrolled_at
            FROM identity
            INNER JOIN identity_enrollment ON
              identity.identifier = identity_enrollment.identifier
            INNER JOIN named_identity ON
              identity.identifier = named_identity.identifier
            WHERE
              named_identity.name = $1
            "#,
        )
        .bind(name);
        let result: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|_| true).unwrap_or(false))
    }
}

#[derive(FromRow)]
pub struct EnrollmentRow {
    identifier: String,
    name: String,
    email: Nullable<String>,
    is_default: Boolean,
    enrolled_at: Nullable<i64>,
}

impl EnrollmentRow {
    fn identity_enrollment(&self) -> Result<IdentityEnrollment> {
        let identifier = Identifier::from_str(self.identifier.as_str())?;
        let email = self
            .email
            .to_option()
            .map(|e| EmailAddress::parse(e.as_str()))
            .transpose()?;

        Ok(IdentityEnrollment::new(
            identifier,
            self.name.clone(),
            self.is_default.to_bool(),
            self.enrolled_at(),
            email,
        ))
    }

    fn enrolled_at(&self) -> Option<OffsetDateTime> {
        self.enrolled_at
            .to_option()
            .map(|at| OffsetDateTime::from_unix_timestamp(at).unwrap_or(OffsetDateTime::now_utc()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ockam::identity::{
        identities, ChangeHistoryRepository, ChangeHistorySqlxDatabase, Identity,
    };

    use crate::cli_state::{EnrollmentsRepository, IdentitiesRepository, IdentitiesSqlxDatabase};

    use super::*;

    #[tokio::test]
    async fn test_identities_enrollment_repository() -> Result<()> {
        let db = create_database().await?;
        let repository = create_repository(db.clone());

        // create some identities
        let identity1 = create_identity(db.clone(), "identity1").await?;
        create_identity(db.clone(), "identity2").await?;

        let email = EmailAddress::parse("test@example.com")?;

        // an identity can be enrolled
        repository
            .set_as_enrolled(identity1.identifier(), &email)
            .await?;

        // retrieve the identities and their enrollment status
        let result = repository.get_all_identities_enrollments().await?;
        assert_eq!(result.len(), 2);

        // retrieve only the enrolled identities
        let result = repository.get_enrolled_identities().await?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status().email(), Some(email).as_ref());

        // the first identity must be seen as enrolled
        let result = repository.is_identity_enrolled("identity1").await?;
        assert!(result);

        // the first identity has been set as the default one when it has been created
        // so we should retrieve this information via is_default_identity_enrolled
        let result = repository.is_default_identity_enrolled().await?;
        assert!(result);

        Ok(())
    }

    /// HELPERS
    async fn create_identity(db: SqlxDatabase, name: &str) -> Result<Identity> {
        let identities = identities().await?;
        let identifier = identities.identities_creation().create_identity().await?;
        let identity = identities.get_identity(&identifier).await?;
        store_identity(db, name, identity).await
    }

    async fn store_identity(db: SqlxDatabase, name: &str, identity: Identity) -> Result<Identity> {
        let change_history_repository = create_change_history_repository(db.clone()).await?;
        let identities_repository = create_identities_repository(db).await?;
        change_history_repository
            .store_change_history(identity.identifier(), identity.change_history().clone())
            .await?;

        identities_repository
            .store_named_identity(identity.identifier(), name, "vault")
            .await?;
        if name == "identity1" {
            identities_repository
                .set_as_default_by_identifier(identity.identifier())
                .await?;
        }
        Ok(identity)
    }

    fn create_repository(db: SqlxDatabase) -> Arc<dyn EnrollmentsRepository> {
        Arc::new(EnrollmentsSqlxDatabase::new(db))
    }

    async fn create_database() -> Result<SqlxDatabase> {
        SqlxDatabase::in_memory("enrollments-test").await
    }

    async fn create_change_history_repository(
        db: SqlxDatabase,
    ) -> Result<Arc<dyn ChangeHistoryRepository>> {
        Ok(Arc::new(ChangeHistorySqlxDatabase::new(db)))
    }

    async fn create_identities_repository(
        db: SqlxDatabase,
    ) -> Result<Arc<dyn IdentitiesRepository>> {
        Ok(Arc::new(IdentitiesSqlxDatabase::new(db)))
    }
}
