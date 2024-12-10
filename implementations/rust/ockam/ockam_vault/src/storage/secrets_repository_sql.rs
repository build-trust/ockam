use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::any::AnyArgumentBuffer;
use sqlx::*;
use std::sync::Arc;
use tracing::debug;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::storage::secrets_repository::SecretsRepository;
use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

use crate::{
    AeadSecret, AeadSecretKeyHandle, ECDSASHA256CurveP256SecretKey, EdDSACurve25519SecretKey,
    HandleToSecret, SigningSecret, SigningSecretKeyHandle, X25519SecretKey, X25519SecretKeyHandle,
    AEAD_TYPE,
};

/// Implementation of a secrets repository using a SQL database
#[derive(Clone)]
pub struct SecretsSqlxDatabase {
    database: SqlxDatabase,
}

impl SecretsSqlxDatabase {
    /// Create a new database for secrets
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for secrets");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn SecretsRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database for secrets
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("secrets").await?))
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

        let query = query(
            r#"
            INSERT INTO signing_secret (handle, secret_type, secret)
            VALUES ($1, $2, $3)
            ON CONFLICT (handle)
            DO UPDATE SET secret_type = $2, secret = $3"#,
        )
        .bind(handle)
        .bind(secret_type)
        .bind(secret);
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete_signing_secret(&self, handle: &SigningSecretKeyHandle) -> Result<bool> {
        let query = query("DELETE FROM signing_secret WHERE handle = $1").bind(handle);
        let res = query.execute(&*self.database.pool).await.into_core()?;

        Ok(res.rows_affected() != 0)
    }

    async fn get_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> Result<Option<SigningSecret>> {
        let query =
            query_as("SELECT handle, secret_type, secret FROM signing_secret WHERE handle = $1")
                .bind(handle);
        let row: Option<SigningSecretRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.signing_secret()).transpose()?)
    }

    async fn get_signing_secret_handles(&self) -> Result<Vec<SigningSecretKeyHandle>> {
        let query = query_as("SELECT handle, secret_type, secret FROM signing_secret");
        let rows: Vec<SigningSecretRow> =
            query.fetch_all(&*self.database.pool).await.into_core()?;
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
        let query = query(
            r#"
        INSERT INTO x25519_secret (handle, secret)
        VALUES ($1, $2)
        ON CONFLICT (handle)
        DO UPDATE SET secret = $2"#,
        )
        .bind(handle)
        .bind(secret);
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<bool> {
        let query = query("DELETE FROM x25519_secret WHERE handle = $1").bind(handle);
        let res = query.execute(&*self.database.pool).await.into_core()?;

        Ok(res.rows_affected() != 0)
    }

    async fn get_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> Result<Option<X25519SecretKey>> {
        let query =
            query_as("SELECT handle, secret FROM x25519_secret WHERE handle = $1").bind(handle);
        let row: Option<X25519SecretRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.x25519_secret()).transpose()?)
    }

    async fn get_x25519_secret_handles(&self) -> Result<Vec<X25519SecretKeyHandle>> {
        let query = query_as("SELECT handle, secret FROM x25519_secret");
        let rows: Vec<X25519SecretRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        Ok(rows
            .iter()
            .map(|r| r.handle())
            .collect::<Result<Vec<_>>>()?)
    }

    async fn store_aead_secret(
        &self,
        handle: &AeadSecretKeyHandle,
        secret: AeadSecret,
    ) -> Result<()> {
        let query = query(
            r#"
                INSERT INTO aead_secret (handle, type, secret)
                VALUES ($1, $2, $3)
                ON CONFLICT (handle)
                DO UPDATE SET type = $2, secret = $3"#,
        )
        .bind(handle)
        .bind(AEAD_TYPE)
        .bind(secret);
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<bool> {
        let query = query("DELETE FROM aead_secret WHERE handle = $1").bind(handle);
        let res = query.execute(&*self.database.pool).await.into_core()?;

        Ok(res.rows_affected() != 0)
    }

    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<Option<AeadSecret>> {
        let query = query_as("SELECT secret FROM aead_secret WHERE handle = $1 AND type = $2")
            .bind(handle)
            .bind(AEAD_TYPE);
        let row: Option<AeadSecretRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.aead_secret()).transpose()?)
    }

    async fn delete_all(&self) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query("DELETE FROM signing_secret");
        query1.execute(&mut *transaction).await.void()?;

        let query2 = query("DELETE FROM x25519_secret");
        query2.execute(&mut *transaction).await.void()?;

        let query3 = query("DELETE FROM aead_secret");
        query3.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }
}

impl Type<Any> for SigningSecret {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for SigningSecret {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'_, Any>>::encode_by_ref(&self.key().to_vec(), buf)
    }
}

impl Type<Any> for SigningSecretKeyHandle {
    fn type_info() -> <Any as Database>::TypeInfo {
        <HandleToSecret as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for SigningSecretKeyHandle {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <HandleToSecret as Encode<'_, Any>>::encode_by_ref(self.handle(), buf)
    }
}

impl Type<Any> for HandleToSecret {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for HandleToSecret {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'_, Any>>::encode_by_ref(self.value(), buf)
    }
}

impl Type<Any> for X25519SecretKeyHandle {
    fn type_info() -> <Any as Database>::TypeInfo {
        <HandleToSecret as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for X25519SecretKeyHandle {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <HandleToSecret as Encode<'_, Any>>::encode_by_ref(&self.0, buf)
    }
}

impl Type<Any> for AeadSecretKeyHandle {
    fn type_info() -> <Any as Database>::TypeInfo {
        <HandleToSecret as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for AeadSecretKeyHandle {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <HandleToSecret as Encode<'_, Any>>::encode_by_ref(&self.0 .0, buf)
    }
}

impl Type<Any> for X25519SecretKey {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for X25519SecretKey {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'_, Any>>::encode_by_ref(&self.key().to_vec(), buf)
    }
}

impl Type<Any> for AeadSecret {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for AeadSecret {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'_, Any>>::encode_by_ref(&self.0.to_vec(), buf)
    }
}

#[derive(FromRow, Zeroize, ZeroizeOnDrop)]
struct SigningSecretRow {
    handle: Vec<u8>,
    secret_type: String,
    secret: Vec<u8>,
}

impl SigningSecretRow {
    fn signing_secret(&self) -> Result<SigningSecret> {
        let secret = self.secret.clone().try_into().map_err(|_| {
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

#[derive(FromRow, Zeroize, ZeroizeOnDrop)]
struct X25519SecretRow {
    handle: Vec<u8>,
    secret: Vec<u8>,
}

impl X25519SecretRow {
    fn x25519_secret(&self) -> Result<X25519SecretKey> {
        let secret = self.secret.as_slice().try_into().map_err(|_| {
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

#[derive(FromRow, Zeroize, ZeroizeOnDrop)]
struct AeadSecretRow {
    secret: Vec<u8>,
}

impl AeadSecretRow {
    fn aead_secret(&self) -> Result<AeadSecret> {
        let secret = self.secret.as_slice().try_into().map_err(|_| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                "cannot convert an AEAD secret to array",
            )
        })?;

        Ok(AeadSecret(secret))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use ockam_core::compat::sync::Arc;

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

    #[tokio::test]
    async fn test_aead_secrets_repository() -> Result<()> {
        let repository = create_repository().await?;

        let handle1 = AeadSecretKeyHandle::new(HandleToSecret::new(vec![1, 2, 3]));
        let secret1 = AeadSecret([1; 32]);

        let handle2 = AeadSecretKeyHandle::new(HandleToSecret::new(vec![4, 5, 6]));
        let secret2 = AeadSecret([2; 32]);

        repository
            .store_aead_secret(&handle1, secret1.clone())
            .await?;
        repository
            .store_aead_secret(&handle2, secret2.clone())
            .await?;

        let result = repository.get_aead_secret(&handle1).await?;
        assert!(result == Some(secret1));

        let result = repository.get_aead_secret(&handle2).await?;
        assert!(result == Some(secret2));

        repository.delete_aead_secret(&handle1).await?;

        let result = repository.get_aead_secret(&handle1).await?;
        assert!(result.is_none());

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn SecretsRepository>> {
        Ok(Arc::new(SecretsSqlxDatabase::create().await?))
    }
}
