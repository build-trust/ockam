use ockam::identity::TimestampInSeconds;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::*;
use sqlx_core::any::AnyArgumentBuffer;
use std::sync::Arc;

use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{
    AuthorityEnrollmentTokenRepository, EnrollmentToken, EnrollmentTokenRow,
};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};
use tracing::debug;

/// Implementation of [`AuthorityEnrollmentTokenRepository`] trait based on an underlying database
/// using sqlx as its API
#[derive(Clone)]
pub struct AuthorityEnrollmentTokenSqlxDatabase {
    database: SqlxDatabase,
}

impl AuthorityEnrollmentTokenSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for authority enrollment tokens");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn AuthorityEnrollmentTokenRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("authority enrollment tokens").await?,
        ))
    }
}

#[async_trait]
impl AuthorityEnrollmentTokenRepository for AuthorityEnrollmentTokenSqlxDatabase {
    async fn use_token(
        &self,
        one_time_code: OneTimeCode,
        now: TimestampInSeconds,
    ) -> Result<Option<EnrollmentToken>> {
        // We need to delete expired tokens regularly
        // Also makes sure we don't get expired tokens later inside this function
        let query1 = query("DELETE FROM authority_enrollment_token WHERE expires_at <= $1")
            .bind(now.0 as i64);

        let res = query1.execute(&*self.database.pool).await.into_core()?;
        debug!("Deleted {} expired enrollment tokens", res.rows_affected());

        let mut transaction = self.database.pool.begin().await.into_core()?;

        let query2 = query_as("SELECT one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes FROM authority_enrollment_token WHERE one_time_code = $1")
            .bind(one_time_code);
        let row: Option<EnrollmentTokenRow> =
            query2.fetch_optional(&mut *transaction).await.into_core()?;
        let token: Option<EnrollmentToken> = row.map(|r| r.try_into()).transpose()?;

        if let Some(token) = &token {
            if token.ttl_count <= 1 {
                let query3 =
                    query("DElETE FROM authority_enrollment_token WHERE one_time_code = $1")
                        .bind(one_time_code);
                query3.execute(&mut *transaction).await.void()?;
                debug!(
                    "Deleted enrollment token because it has been used. Reference: {}",
                    token.reference()
                );
            } else {
                let new_ttl_count = token.ttl_count - 1;
                let query3 = query(
                    "UPDATE authority_enrollment_token SET ttl_count = $1 WHERE one_time_code = $2",
                )
                .bind(new_ttl_count as i64)
                .bind(one_time_code);
                query3.execute(&mut *transaction).await.void()?;
                debug!(
                    "Decreasing enrollment token usage count to {}. Reference: {}",
                    new_ttl_count,
                    token.reference()
                );
            }
        }

        transaction.commit().await.void()?;

        Ok(token)
    }

    async fn store_new_token(&self, token: EnrollmentToken) -> Result<()> {
        let query = query(
            r#"
            INSERT INTO authority_enrollment_token (one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (one_time_code)
            DO UPDATE SET reference = $2, issued_by = $3, created_at = $4, expires_at = $5, ttl_count = $6, attributes = $7"#,
        )
        .bind(token.one_time_code)
        .bind(token.reference)
        .bind(token.issued_by)
        .bind(token.created_at)
        .bind(token.expires_at)
        .bind(token.ttl_count as i64)
        .bind(ockam_core::cbor_encode_preallocate(token.attrs)?);

        query.execute(&*self.database.pool).await.void()
    }
}

// Database serialization / deserialization

impl Type<Any> for OneTimeCode {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl sqlx::Encode<'_, Any> for OneTimeCode {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as sqlx::Encode<'_, Any>>::encode_by_ref(&String::from(self), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::utils::now;
    use ockam::identity::Identifier;
    use ockam_core::compat::sync::Arc;
    use ockam_node::database::with_dbs;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use std::time::Duration;

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_one_time_token() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityEnrollmentTokenRepository> =
                Arc::new(AuthorityEnrollmentTokenSqlxDatabase::new(db));

            let one_time_code = OneTimeCode::new();

            let issued_by = Identifier::from_str(
                "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .unwrap();

            let created_at = now()?;
            let expires_at = created_at + 10;

            let mut attrs = BTreeMap::<String, String>::default();
            attrs.insert("role".to_string(), "user".to_string());

            let token = EnrollmentToken {
                one_time_code,
                reference: None,
                issued_by: issued_by.clone(),
                created_at,
                expires_at,
                ttl_count: 1,
                attrs: attrs.clone(),
            };

            repository.store_new_token(token).await?;

            let token1 = repository.use_token(one_time_code, now()?).await?;
            assert!(token1.is_some());
            let token1 = token1.unwrap();
            assert_eq!(token1.one_time_code, one_time_code);
            assert_eq!(token1.reference, None);
            assert_eq!(token1.issued_by, issued_by);
            assert_eq!(token1.created_at, created_at);
            assert_eq!(token1.expires_at, expires_at);
            assert_eq!(token1.ttl_count, 1);
            assert_eq!(token1.attrs, attrs);

            let token2 = repository.use_token(one_time_code, now()?).await?;
            assert!(token2.is_none());

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_with_reference() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityEnrollmentTokenRepository> =
                Arc::new(AuthorityEnrollmentTokenSqlxDatabase::new(db));

            let one_time_code = OneTimeCode::new();
            let reference = Some(String::from(&OneTimeCode::new()));

            let issued_by = Identifier::from_str(
                "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .unwrap();

            let created_at = now()?;
            let expires_at = created_at + 10;

            let mut attrs = BTreeMap::<String, String>::default();
            attrs.insert("role".to_string(), "user".to_string());

            let token = EnrollmentToken {
                one_time_code,
                reference: reference.clone(),
                issued_by: issued_by.clone(),
                created_at,
                expires_at,
                ttl_count: 1,
                attrs: attrs.clone(),
            };

            repository.store_new_token(token).await?;

            let token1 = repository.use_token(one_time_code, now()?).await?;
            assert!(token1.is_some());
            let token1 = token1.unwrap();
            assert_eq!(token1.one_time_code, one_time_code);
            assert_eq!(token1.reference, reference);
            assert_eq!(token1.issued_by, issued_by);
            assert_eq!(token1.created_at, created_at);
            assert_eq!(token1.expires_at, expires_at);
            assert_eq!(token1.ttl_count, 1);
            assert_eq!(token1.attrs, attrs);

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_two_time_token() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityEnrollmentTokenRepository> =
                Arc::new(AuthorityEnrollmentTokenSqlxDatabase::new(db));

            let one_time_code = OneTimeCode::new();

            let issued_by = Identifier::from_str(
                "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .unwrap();

            let created_at = now()?;
            let expires_at = created_at + 10;

            let mut attrs = BTreeMap::<String, String>::default();
            attrs.insert("role".to_string(), "user".to_string());

            let token = EnrollmentToken {
                one_time_code,
                reference: None,
                issued_by: issued_by.clone(),
                created_at,
                expires_at,
                ttl_count: 2,
                attrs: attrs.clone(),
            };

            repository.store_new_token(token).await?;

            let token1 = repository.use_token(one_time_code, now()?).await?;
            let token2 = repository.use_token(one_time_code, now()?).await?;
            let token3 = repository.use_token(one_time_code, now()?).await?;
            assert!(token1.is_some());
            assert!(token2.is_some());
            assert!(token3.is_none());

            let token1 = token1.unwrap();
            let token2 = token2.unwrap();

            assert_eq!(token1.reference, token2.reference);
            assert_eq!(token1.one_time_code, token2.one_time_code);
            assert_eq!(token1.issued_by, token2.issued_by);
            assert_eq!(token1.created_at, token2.created_at);
            assert_eq!(token1.expires_at, token2.expires_at);
            assert_eq!(token1.attrs, token2.attrs);

            assert_eq!(token1.ttl_count, 2);
            assert_eq!(token2.ttl_count, 1);

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_expired_token() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn AuthorityEnrollmentTokenRepository> =
                Arc::new(AuthorityEnrollmentTokenSqlxDatabase::new(db));

            let one_time_code = OneTimeCode::new();

            let issued_by = Identifier::from_str(
                "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .unwrap();

            let created_at = now()?;
            let expires_at = created_at + 1;

            let mut attrs = BTreeMap::<String, String>::default();
            attrs.insert("role".to_string(), "user".to_string());

            let token = EnrollmentToken {
                one_time_code,
                reference: None,
                issued_by: issued_by.clone(),
                created_at,
                expires_at,
                ttl_count: 1,
                attrs: attrs.clone(),
            };

            repository.store_new_token(token.clone()).await?;
            // Try to store the same token twice
            repository.store_new_token(token).await?;

            tokio::time::sleep(Duration::from_secs(2)).await;

            let token1 = repository.use_token(one_time_code, now()?).await?;
            assert!(token1.is_none());
            Ok(())
        })
        .await
    }
}
