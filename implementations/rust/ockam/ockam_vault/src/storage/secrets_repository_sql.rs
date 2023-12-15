use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::storage::secrets_repository::SecretsRepository;

use crate::{
    ECDSASHA256CurveP256SecretKey, EdDSACurve25519SecretKey, HandleToSecret, SigningSecret,
    SigningSecretKeyHandle, X25519SecretKey, X25519SecretKeyHandle,
};

/// Implementation of a secrets repository using a SQL database
#[derive(Clone)]
pub struct SecretsSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl SecretsSqlxDatabase {
    /// Create a new database for policies keys
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for secrets");
        Self { database }
    }

    /// Create a new in-memory database for policies
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("secrets").await?,
        )))
    }
}

const ED_DSA_CURVE_25519: &str = "EdDSACurve25519";
const EC_DSA_SHA256_CURVE_P256: &str = "ECDSASHA256CurveP256";

#[async_trait]
impl SecretsRepository for SecretsSqlxDatabase {
    async fn store_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
        secret: SigningSecret,
    ) -> Result<()> {
        let secret_type: String = match handle {
            SigningSecretKeyHandle::EdDSACurve25519(_) => ED_DSA_CURVE_25519.into(),
            SigningSecretKeyHandle::ECDSASHA256CurveP256(_) => EC_DSA_SHA256_CURVE_P256.into(),
        };

        let query = query("INSERT OR REPLACE INTO signing_secret VALUES (?, ?, ?)")
            .bind(handle.to_sql())
            .bind(secret_type.to_sql())
            .bind(secret.to_sql());
        query.execute(&self.database.pool).await.void()
    }

    async fn delete_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> Result<Option<SigningSecret>> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 =
            query_as("SELECT handle, secret_type, secret FROM signing_secret WHERE handle=?")
                .bind(handle.to_sql());
        let row: Option<SigningSecretRow> =
            query1.fetch_optional(&mut *transaction).await.into_core()?;
        let secret = row.map(|r| r.signing_secret()).transpose()?;

        let result = if let Some(secret) = secret {
            let query = query("DELETE FROM signing_secret WHERE handle = ?").bind(handle.to_sql());
            query.execute(&mut *transaction).await.void()?;
            Some(secret)
        } else {
            None
        };
        transaction.commit().await.void()?;
        Ok(result)
    }

    async fn get_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> Result<Option<SigningSecret>> {
        let query =
            query_as("SELECT handle, secret_type, secret FROM signing_secret WHERE handle=?")
                .bind(handle.to_sql());
        let row: Option<SigningSecretRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.signing_secret()).transpose()?)
    }

    async fn get_signing_secret_handles(&self) -> Result<Vec<SigningSecretKeyHandle>> {
        let query = query_as("SELECT handle, secret_type, secret FROM signing_secret");
        let rows: Vec<SigningSecretRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        Ok(rows
            .iter()
            .map(|r| r.handle())
            .collect::<Result<Vec<_>>>()?)
    }

    async fn store_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
        secret: X25519SecretKey,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO x25519_secret VALUES (?, ?)")
            .bind(handle.to_sql())
            .bind(secret.to_sql());
        query.execute(&self.database.pool).await.void()
    }

    async fn delete_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> Result<Option<X25519SecretKey>> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query_as("SELECT handle, secret FROM x25519_secret WHERE handle=?")
            .bind(handle.to_sql());
        let row: Option<X25519SecretRow> =
            query1.fetch_optional(&mut *transaction).await.into_core()?;
        let secret = row.map(|r| r.x25519_secret()).transpose()?;

        let result = if let Some(secret) = secret {
            let query = query("DELETE FROM x25519_secret WHERE handle = ?").bind(handle.to_sql());
            query.execute(&mut *transaction).await.void()?;
            Some(secret)
        } else {
            None
        };
        transaction.commit().await.void()?;
        Ok(result)
    }

    async fn get_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> Result<Option<X25519SecretKey>> {
        let query = query_as("SELECT handle, secret FROM x25519_secret WHERE handle=?")
            .bind(handle.to_sql());
        let row: Option<X25519SecretRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.x25519_secret()).transpose()?)
    }

    async fn get_x25519_secret_handles(&self) -> Result<Vec<X25519SecretKeyHandle>> {
        let query = query_as("SELECT handle, secret FROM x25519_secret");
        let rows: Vec<X25519SecretRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        Ok(rows
            .iter()
            .map(|r| r.handle())
            .collect::<Result<Vec<_>>>()?)
    }

    async fn delete_all(&self) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query("DELETE FROM signing_secret");
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM x25519_secret");
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }
}

impl ToSqlxType for SigningSecret {
    fn to_sql(&self) -> SqlxType {
        match self {
            SigningSecret::EdDSACurve25519(k) => k.key().to_sql(),
            SigningSecret::ECDSASHA256CurveP256(k) => k.key().to_sql(),
        }
    }
}

impl ToSqlxType for SigningSecretKeyHandle {
    fn to_sql(&self) -> SqlxType {
        self.handle().to_sql()
    }
}

impl ToSqlxType for X25519SecretKeyHandle {
    fn to_sql(&self) -> SqlxType {
        self.0.value().to_sql()
    }
}

impl ToSqlxType for HandleToSecret {
    fn to_sql(&self) -> SqlxType {
        self.value().to_sql()
    }
}

impl ToSqlxType for X25519SecretKey {
    fn to_sql(&self) -> SqlxType {
        self.key().to_sql()
    }
}

#[derive(FromRow)]
struct SigningSecretRow {
    handle: Vec<u8>,
    secret_type: String,
    secret: Vec<u8>,
}

impl SigningSecretRow {
    fn signing_secret(&self) -> Result<SigningSecret> {
        let secret: [u8; 32] = self.secret.clone().try_into().map_err(|_| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                "cannot convert a signing secret to [u8; 32]",
            )
        })?;
        match self.secret_type.as_str() {
            "EdDSACurve25519" => Ok(SigningSecret::EdDSACurve25519(
                EdDSACurve25519SecretKey::new(secret),
            )),
            "ECDSASHA256CurveP256" => Ok(SigningSecret::ECDSASHA256CurveP256(
                ECDSASHA256CurveP256SecretKey::new(secret),
            )),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                "cannot deserialize a signing secret",
            )),
        }
    }

    fn handle(&self) -> Result<SigningSecretKeyHandle> {
        match self.secret_type.as_str() {
            "EdDSACurve25519" => Ok(SigningSecretKeyHandle::EdDSACurve25519(
                HandleToSecret::new(self.handle.clone()),
            )),
            "ECDSASHA256CurveP256" => Ok(SigningSecretKeyHandle::ECDSASHA256CurveP256(
                HandleToSecret::new(self.handle.clone()),
            )),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                "cannot deserialize a signing secret handle",
            )),
        }
    }
}

#[derive(FromRow)]
struct X25519SecretRow {
    handle: Vec<u8>,
    secret: Vec<u8>,
}

impl X25519SecretRow {
    fn x25519_secret(&self) -> Result<X25519SecretKey> {
        let secret: [u8; 32] = self.secret.clone().try_into().map_err(|_| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                "cannot convert a X25519 secret to [u8; 32]",
            )
        })?;
        Ok(X25519SecretKey::new(secret))
    }

    fn handle(&self) -> Result<X25519SecretKeyHandle> {
        Ok(X25519SecretKeyHandle(HandleToSecret::new(
            self.handle.clone(),
        )))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_signing_secrets_repository() -> Result<()> {
        let repository = create_repository().await?;

        let handle1 =
            SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(vec![1, 2, 3]));
        let secret1 =
            SigningSecret::ECDSASHA256CurveP256(ECDSASHA256CurveP256SecretKey::new([1; 32]));

        let handle2 = SigningSecretKeyHandle::EdDSACurve25519(HandleToSecret::new(vec![4, 5, 6]));
        let secret2 = SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new([1; 32]));

        repository
            .store_signing_secret(&handle1, secret1.clone())
            .await?;
        repository
            .store_signing_secret(&handle2, secret2.clone())
            .await?;

        let result = repository.get_signing_secret(&handle1).await?;
        assert!(result == Some(secret1));

        let result = repository.get_signing_secret_handles().await?;
        assert_eq!(result, vec![handle1.clone(), handle2]);

        repository.delete_signing_secret(&handle1).await?;

        let result = repository.get_signing_secret(&handle1).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_x25519_secrets_repository() -> Result<()> {
        let repository = create_repository().await?;

        let handle1 = X25519SecretKeyHandle(HandleToSecret::new(vec![1, 2, 3]));
        let secret1 = X25519SecretKey::new([1; 32]);

        let handle2 = X25519SecretKeyHandle(HandleToSecret::new(vec![4, 5, 6]));
        let secret2 = X25519SecretKey::new([1; 32]);

        repository
            .store_x25519_secret(&handle1, secret1.clone())
            .await?;
        repository
            .store_x25519_secret(&handle2, secret2.clone())
            .await?;

        let result = repository.get_x25519_secret(&handle1).await?;
        assert!(result == Some(secret1));

        let result = repository.get_x25519_secret_handles().await?;
        assert_eq!(result, vec![handle1.clone(), handle2]);

        repository.delete_x25519_secret(&handle1).await?;

        let result = repository.get_x25519_secret(&handle1).await?;
        assert!(result.is_none());

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn SecretsRepository>> {
        Ok(SecretsSqlxDatabase::create().await?)
    }
}
