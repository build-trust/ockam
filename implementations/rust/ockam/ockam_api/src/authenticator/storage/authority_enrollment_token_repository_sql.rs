use ockam::identity::TimestampInSeconds;
use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::authenticator::one_time_code::OneTimeCode;
use crate::authenticator::{
    AuthorityEnrollmentTokenRepository, EnrollmentToken, EnrollmentTokenRow,
};

/// Implementation of [`AuthorityEnrollmentTokenRepository`] trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
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
        let query1 =
            query("DELETE FROM authority_enrollment_token WHERE expires_at<=?").bind(now.to_sql());

        let res = query1.execute(&*self.database.pool).await.into_core()?;
        debug!("Deleted {} expired enrollment tokens", res.rows_affected());

        let mut transaction = self.database.pool.begin().await.into_core()?;

        let query2 = query_as("SELECT one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes FROM authority_enrollment_token WHERE one_time_code=?")
            .bind(one_time_code.to_sql());
        let row: Option<EnrollmentTokenRow> =
            query2.fetch_optional(&mut *transaction).await.into_core()?;
        let token: Option<EnrollmentToken> = row.map(|r| r.try_into()).transpose()?;

        if let Some(token) = &token {
            if token.ttl_count <= 1 {
                let query3 = query("DElETE FROM authority_enrollment_token WHERE one_time_code=?")
                    .bind(one_time_code.to_sql());
                query3.execute(&mut *transaction).await.void()?;
                debug!(
                    "Deleted enrollment token because it has been used. Reference: {}",
                    token.reference()
                );
            } else {
                let new_ttl_count = token.ttl_count - 1;
                let query3 = query(
                    "UPDATE authority_enrollment_token SET ttl_count=? WHERE one_time_code=?",
                )
                .bind(new_ttl_count as i64)
                .bind(one_time_code.to_sql());
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
            "INSERT OR REPLACE INTO authority_enrollment_token (one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(token.one_time_code.to_sql())
        .bind(token.reference.map(|r| r.to_sql()))
        .bind(token.issued_by.to_sql())
        .bind(token.created_at.to_sql())
        .bind(token.expires_at.to_sql())
        .bind(token.ttl_count.to_sql())
        .bind(ockam_core::cbor_encode_preallocate(token.attrs)?.to_sql());

        query.execute(&*self.database.pool).await.void()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::utils::now;
    use ockam::identity::Identifier;
    use ockam_core::compat::sync::Arc;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use std::time::Duration;

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_one_time_token() -> Result<()> {
        let repository = create_repository().await?;

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
            one_time_code: one_time_code.clone(),
            reference: None,
            issued_by: issued_by.clone(),
            created_at,
            expires_at,
            ttl_count: 1,
            attrs: attrs.clone(),
        };

        repository.store_new_token(token).await?;

        let token1 = repository.use_token(one_time_code.clone(), now()?).await?;
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
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_with_reference() -> Result<()> {
        let repository = create_repository().await?;

        let one_time_code = OneTimeCode::new();
        let reference = Some(OneTimeCode::new().to_string());

        let issued_by = Identifier::from_str(
            "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        let created_at = now()?;
        let expires_at = created_at + 10;

        let mut attrs = BTreeMap::<String, String>::default();
        attrs.insert("role".to_string(), "user".to_string());

        let token = EnrollmentToken {
            one_time_code: one_time_code.clone(),
            reference: reference.clone(),
            issued_by: issued_by.clone(),
            created_at,
            expires_at,
            ttl_count: 1,
            attrs: attrs.clone(),
        };

        repository.store_new_token(token).await?;

        let token1 = repository.use_token(one_time_code.clone(), now()?).await?;
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
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_two_time_token() -> Result<()> {
        let repository = create_repository().await?;

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
            one_time_code: one_time_code.clone(),
            reference: None,
            issued_by: issued_by.clone(),
            created_at,
            expires_at,
            ttl_count: 2,
            attrs: attrs.clone(),
        };

        repository.store_new_token(token).await?;

        let token1 = repository.use_token(one_time_code.clone(), now()?).await?;
        let token2 = repository.use_token(one_time_code.clone(), now()?).await?;
        let token3 = repository.use_token(one_time_code.clone(), now()?).await?;
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
    }

    #[tokio::test]
    async fn test_authority_enrollment_token_repository_expired_token() -> Result<()> {
        let repository = create_repository().await?;

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
            one_time_code: one_time_code.clone(),
            reference: None,
            issued_by: issued_by.clone(),
            created_at,
            expires_at,
            ttl_count: 1,
            attrs: attrs.clone(),
        };

        repository.store_new_token(token).await?;

        tokio::time::sleep(Duration::from_secs(2)).await;

        let token1 = repository.use_token(one_time_code.clone(), now()?).await?;
        assert!(token1.is_none());

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn AuthorityEnrollmentTokenRepository>> {
        Ok(Arc::new(
            AuthorityEnrollmentTokenSqlxDatabase::create().await?,
        ))
    }
}
