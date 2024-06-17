use core::str::FromStr;

use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::identity::IdentityConstants;
use crate::models::{Identifier, PurposeKeyAttestation};
use crate::purpose_keys::storage::PurposeKeysRepository;
use crate::Purpose;

/// Storage for own [`super::super::super::purpose_key::PurposeKey`]s
#[derive(Clone)]
pub struct PurposeKeysSqlxDatabase {
    database: SqlxDatabase,
}

impl PurposeKeysSqlxDatabase {
    /// Create a new database for purpose keys
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for purpose keys");
        Self { database }
    }

    /// Create a new in-memory database for purpose keys
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("purpose keys").await?))
    }
}

#[async_trait]
impl PurposeKeysRepository for PurposeKeysSqlxDatabase {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO purpose_key VALUES (?, ?, ?)")
            .bind(subject.to_sql())
            .bind(purpose.to_sql())
            .bind(ockam_core::cbor_encode_preallocate(purpose_key_attestation)?.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()> {
        let query = query("DELETE FROM purpose_key WHERE identifier = ? and purpose = ?")
            .bind(subject.to_sql())
            .bind(purpose.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>> {
        let query = query_as("SELECT identifier, purpose, purpose_key_attestation FROM purpose_key WHERE identifier=$1 and purpose=$2")
            .bind(identifier.to_sql())
            .bind(purpose.to_sql());
        let row: Option<PurposeKeyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.purpose_key_attestation()).transpose()?)
    }

    async fn delete_all(&self) -> Result<()> {
        query("DELETE FROM purpose_key")
            .execute(&*self.database.pool)
            .await
            .void()
    }
}

// Database serialization / deserialization

#[derive(FromRow)]
pub(crate) struct PurposeKeyRow {
    // The identifier who is using this key
    identifier: String,
    // Purpose of the key (signing, encrypting, etc...)
    purpose: String,
    // Attestation that this key is valid
    purpose_key_attestation: Vec<u8>,
}

impl PurposeKeyRow {
    #[allow(dead_code)]
    pub(crate) fn identifier(&self) -> Result<Identifier> {
        Identifier::from_str(&self.identifier)
    }

    #[allow(dead_code)]
    pub(crate) fn purpose(&self) -> Result<Purpose> {
        match self.purpose.as_str() {
            IdentityConstants::SECURE_CHANNEL_PURPOSE_KEY => Ok(Purpose::SecureChannel),
            IdentityConstants::CREDENTIALS_PURPOSE_KEY => Ok(Purpose::Credentials),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("unknown purpose {}", self.purpose),
            )),
        }
    }

    pub(crate) fn purpose_key_attestation(&self) -> Result<PurposeKeyAttestation> {
        Ok(minicbor::decode(self.purpose_key_attestation.as_slice())?)
    }
}

impl ToSqlxType for Purpose {
    fn to_sql(&self) -> SqlxType {
        match self {
            Purpose::SecureChannel => {
                SqlxType::Text(IdentityConstants::SECURE_CHANNEL_PURPOSE_KEY.to_string())
            }
            Purpose::Credentials => {
                SqlxType::Text(IdentityConstants::CREDENTIALS_PURPOSE_KEY.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identities;
    use crate::models::PurposeKeyAttestationSignature;
    use ockam_core::compat::sync::Arc;
    use ockam_vault::ECDSASHA256CurveP256Signature;

    #[tokio::test]
    async fn test_purpose_keys_repository() -> Result<()> {
        let repository = create_repository().await?;

        // A purpose key can be stored and retrieved, given the owning identifier and purpose type
        let identity1 = create_identity().await?;
        let attestation1 = PurposeKeyAttestation {
            data: vec![1, 2, 3],
            signature: PurposeKeyAttestationSignature::ECDSASHA256CurveP256(
                ECDSASHA256CurveP256Signature([1; 64]),
            ),
        };
        repository
            .set_purpose_key(&identity1, Purpose::Credentials, &attestation1)
            .await?;

        let result = repository
            .get_purpose_key(&identity1, Purpose::Credentials)
            .await?;
        assert_eq!(result, Some(attestation1));

        // the attestation can be updated
        let attestation2 = PurposeKeyAttestation {
            data: vec![4, 5, 6],
            signature: PurposeKeyAttestationSignature::ECDSASHA256CurveP256(
                ECDSASHA256CurveP256Signature([1; 64]),
            ),
        };
        repository
            .set_purpose_key(&identity1, Purpose::Credentials, &attestation2)
            .await?;

        let result = repository
            .get_purpose_key(&identity1, Purpose::Credentials)
            .await?;
        assert_eq!(result, Some(attestation2));

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn PurposeKeysRepository>> {
        Ok(Arc::new(PurposeKeysSqlxDatabase::create().await?))
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }
}
