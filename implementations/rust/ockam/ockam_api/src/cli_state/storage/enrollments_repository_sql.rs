use std::str::FromStr;

use sqlx::sqlite::SqliteRow;
use sqlx::FromRow;
use sqlx::*;
use time::OffsetDateTime;

use ockam::identity::Identifier;
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::async_trait;
use ockam_core::Result;

use crate::cli_state::enrollments::IdentityEnrollment;
use crate::cli_state::EnrollmentsRepository;
use crate::cloud::email_address::EmailAddress;

#[derive(Clone)]
pub struct EnrollmentsSqlxDatabase {
    database: SqlxDatabase,
}

impl EnrollmentsSqlxDatabase {
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for enrollments");
        Self { database }
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
        let query = query("INSERT OR REPLACE INTO identity_enrollment(identifier, enrolled_at, email) VALUES (?, ?, ?)")
            .bind(identifier.to_sql())
            .bind(OffsetDateTime::now_utc().to_sql())
            .bind(email.to_sql());
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
              named_identity.is_default = ?
            "#,
        )
        .bind(true.to_sql());
        let result: Option<SqliteRow> = query
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
              named_identity.name = ?
            "#,
        )
        .bind(name.to_sql());
        let result: Option<SqliteRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|_| true).unwrap_or(false))
    }
}

#[derive(FromRow)]
pub struct EnrollmentRow {
    identifier: String,
    name: Option<String>,
    email: Option<String>,
    is_default: bool,
    enrolled_at: Option<i64>,
}

impl EnrollmentRow {
    fn identity_enrollment(&self) -> Result<IdentityEnrollment> {
        let identifier = Identifier::from_str(self.identifier.as_str())?;
        let email = self
            .email
            .as_ref()
            .map(|e| EmailAddress::parse(e.as_str()))
            .transpose()?;

        Ok(IdentityEnrollment::new(
            identifier,
            self.name.clone(),
            email,
            self.is_default,
            self.enrolled_at(),
        ))
    }

    fn enrolled_at(&self) -> Option<OffsetDateTime> {
        self.enrolled_at
            .map(|at| OffsetDateTime::from_unix_timestamp(at).unwrap_or(OffsetDateTime::now_utc()))
    }
}

#[cfg(test)]
mod tests {
    use crate::cli_state::{EnrollmentsRepository, IdentitiesRepository, IdentitiesSqlxDatabase};
    use ockam::identity::{
        identities, ChangeHistoryRepository, ChangeHistorySqlxDatabase, Identity,
    };
    use std::sync::Arc;

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
        assert_eq!(result[0].email(), Some(email));

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
