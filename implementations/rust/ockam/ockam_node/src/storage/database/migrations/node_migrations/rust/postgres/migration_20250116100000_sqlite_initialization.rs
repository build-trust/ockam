use crate::database::{Boolean, FromSqlxError, RustMigration, SqlxDatabase, ToVoid, Version};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This struct initialize the Postgres database with local data found in a SQLite instance.
#[derive(Debug)]
pub struct InitializeFromSqlite;

#[async_trait]
impl RustMigration for InitializeFromSqlite {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> Version {
        Self::version()
    }

    async fn migrate(
        &self,
        legacy_sqlite_database: Option<SqlxDatabase>,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        if let Some(db) = legacy_sqlite_database {
            Self::initialize_postgres(db, connection).await
        } else {
            Ok(())
        }
    }
}

impl InitializeFromSqlite {
    /// Migration version
    pub fn version() -> Version {
        Version(20250116100000)
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20250116100000_sqlite_initialization"
    }

    pub(crate) async fn initialize_postgres(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        info!(
            "migrate the data from sqlite at {:?} to postgres",
            sqlite_database
                .path()
                .unwrap_or("no sqlite database path".into())
        );
        Self::migrate_aead_secrets(sqlite_database.clone(), connection).await?;
        Self::migrate_authority_enrollment_tokens(sqlite_database.clone(), connection).await?;
        Self::migrate_credentials(sqlite_database.clone(), connection).await?;
        Self::migrate_identities(sqlite_database.clone(), connection).await?;
        Self::migrate_identity_attributes(sqlite_database.clone(), connection).await?;
        Self::migrate_members(sqlite_database.clone(), connection).await?;
        Self::migrate_named_identities(sqlite_database.clone(), connection).await?;
        Self::migrate_purpose_keys(sqlite_database.clone(), connection).await?;
        Self::migrate_signing_secrets(sqlite_database.clone(), connection).await?;
        Self::migrate_x25519_secrets(sqlite_database.clone(), connection).await?;
        Ok(())
    }

    async fn migrate_aead_secrets(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_secrets = query_as("SELECT handle, type as secret_type, secret FROM aead_secret");

        let secrets: Vec<AeadSecretRow> = get_secrets
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for secret in secrets {
            query("INSERT INTO aead_secret (handle, type, secret) VALUES ($1, $2, $3)")
                .bind(secret.handle)
                .bind(secret.secret_type)
                .bind(secret.secret)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_authority_enrollment_tokens(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_tokens = query_as("SELECT one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes FROM authority_enrollment_token");

        let tokens: Vec<AuthorityEnrollmentTicketRow> = get_tokens
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for token in tokens {
            query("INSERT INTO authority_enrollment_token (one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(token.one_time_code)
                .bind(token.reference)
                .bind(token.issued_by)
                .bind(token.created_at)
                .bind(token.expires_at)
                .bind(token.ttl_count)
                .bind(token.attributes)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_credentials(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_credentials = query_as("SELECT subject_identifier, issuer_identifier, scope, credential, expires_at, node_name FROM credential");

        let credentials: Vec<CredentialRow> = get_credentials
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for credential in credentials {
            query("INSERT INTO credential (subject_identifier, issuer_identifier, scope, credential, expires_at, node_name) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(credential.subject_identifier)
                .bind(credential.issuer_identifier)
                .bind(credential.scope)
                .bind(credential.credential)
                .bind(credential.expires_at)
                .bind(credential.node_name)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_identities(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_identities = query_as("SELECT identifier, change_history FROM identity");

        let identities: Vec<IdentityRow> = get_identities
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for identity in identities {
            query("INSERT INTO identity (identifier, change_history) VALUES ($1, $2)")
                .bind(identity.identifier)
                .bind(identity.change_history)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_identity_attributes(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_attributes = query_as("SELECT identifier, attributes, added, expires, attested_by, node_name FROM identity_attributes");

        let attributes_rows: Vec<IdentityAttributesRow> = get_attributes
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for attributes in attributes_rows {
            query("INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(attributes.identifier)
                .bind(attributes.attributes)
                .bind(attributes.added)
                .bind(attributes.expires)
                .bind(attributes.attested_by)
                .bind(attributes.node_name)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_members(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_members = query_as("SELECT identifier, added_by, added_at, is_pre_trusted, attributes, authority_id FROM authority_member");

        let members: Vec<AuthorityMemberRow> = get_members
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for member in members {
            query(r#"INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes, authority_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 "#)
                .bind(member.identifier)
                .bind(member.added_by)
                .bind(member.added_at)
                .bind(member.is_pre_trusted.to_bool())
                .bind(member.attributes)
                .bind(member.authority_id).execute(&mut *transaction).await.void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    /// We don't migrate the vault name, everything goes to the default vault, and we don't
    /// consider any identity to be the default identity.
    async fn migrate_named_identities(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_identities =
            query_as("SELECT identifier, name, vault_name, is_default FROM named_identity");

        let identities: Vec<NamedIdentityRow> = get_identities
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        let excluded_identities = ["authority", "ockam-opentelemetry-outlet"];
        for identity in identities {
            if !excluded_identities.contains(&identity.name.as_str()) {
                query("INSERT INTO named_identity (identifier, name, vault_name, is_default) VALUES ($1, $2, $3, $4)")
                    .bind(identity.identifier)
                    .bind(identity.name)
                    .bind("default")
                    .bind(false)
                    .execute(&mut *transaction)
                    .await
                    .void()?;
            }
        }
        transaction.commit().await.void()?;

        Ok(())
    }

    async fn migrate_purpose_keys(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_purpose_keys =
            query_as("SELECT identifier, purpose, purpose_key_attestation FROM purpose_key");

        let purpose_keys: Vec<PurposeKeyRow> = get_purpose_keys
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for purpose_key in purpose_keys {
            query("INSERT INTO purpose_key (identifier, purpose, purpose_key_attestation) VALUES ($1, $2, $3)")
                .bind(purpose_key.identifier)
                .bind(purpose_key.purpose)
                .bind(purpose_key.purpose_key_attestation)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_signing_secrets(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_secrets = query_as("SELECT handle, secret_type, secret FROM signing_secret");

        let secrets: Vec<SigningSecretRow> = get_secrets
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for secret in secrets {
            query("INSERT INTO signing_secret (handle, secret_type, secret) VALUES ($1, $2, $3)")
                .bind(secret.handle)
                .bind(secret.secret_type)
                .bind(secret.secret)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn migrate_x25519_secrets(
        sqlite_database: SqlxDatabase,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let get_secrets = query_as("SELECT handle, secret FROM x25519_secret");

        let secrets: Vec<X25519SecretRow> = get_secrets
            .fetch_all(&*sqlite_database.pool)
            .await
            .into_core()?;

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        for secret in secrets {
            query("INSERT INTO x25519_secret (handle, secret) VALUES ($1, $2)")
                .bind(secret.handle)
                .bind(secret.secret)
                .execute(&mut *transaction)
                .await
                .void()?;
        }
        transaction.commit().await.void()?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseConfiguration, MigrationSet, SqlxDatabase};
    use std::collections::BTreeMap;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        if DatabaseConfiguration::postgres()?.is_none() {
            return Ok(());
        };

        // create a sqlite database and insert some data
        let db_file = NamedTempFile::new().unwrap();
        let db_file = db_file.path();
        let sqlite_database = SqlxDatabase::create_sqlite(db_file).await?;
        insert_aead_secrets(sqlite_database.clone()).await?;
        insert_authority_enrollment_tokens(sqlite_database.clone()).await?;
        insert_credentials(sqlite_database.clone()).await?;
        insert_identities(sqlite_database.clone()).await?;
        insert_identity_attributes(sqlite_database.clone()).await?;
        insert_members(sqlite_database.clone()).await?;
        insert_named_identities(sqlite_database.clone()).await?;
        insert_purpose_keys(sqlite_database.clone()).await?;
        insert_signing_secrets(sqlite_database.clone()).await?;
        insert_x25519_secrets(sqlite_database.clone()).await?;

        // clean the existing postgres database
        let postgres_database = SqlxDatabase::create_postgres_no_migration(None).await?;
        postgres_database.drop_all_postgres_tables().await?;

        // create a new postgres database. Note that this does not run
        // migrations. For Postgres the migration are generally executed with the migrate-database command
        let configuration =
            DatabaseConfiguration::postgres_with_legacy_sqlite_path(Some(db_file.to_path_buf()))?
                .unwrap();
        let migration_set = NodeMigrationSet::from_configuration(configuration.clone()).await?;
        let migrator = migration_set.create_migrator()?;
        migrator.migrate(&postgres_database.pool).await?;

        check_aead_secrets(postgres_database.clone()).await?;
        check_authority_enrollment_tokens(postgres_database.clone()).await?;
        check_credentials(postgres_database.clone()).await?;
        check_identities(postgres_database.clone()).await?;
        check_identity_attributes(postgres_database.clone()).await?;
        check_members(postgres_database.clone()).await?;
        check_named_identities(postgres_database.clone()).await?;
        check_purpose_keys(postgres_database.clone()).await?;
        check_signing_secrets(postgres_database.clone()).await?;
        check_x25519_secrets(postgres_database.clone()).await?;

        // migrating a second time should be a no-op
        let migrator = migration_set.create_migrator()?;
        migrator.migrate(&postgres_database.pool).await?;

        Ok(())
    }

    /// HELPERS
    async fn insert_aead_secrets(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                "INSERT INTO aead_secret (handle, type, secret) VALUES ($1, 'secret_{index}', $2)"
            );
            let handle = format!("handle_{index}");
            let secret = format!("secret_{index}");
            let query = query(&q).bind(handle.as_bytes()).bind(secret.as_bytes());
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_aead_secrets(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT handle, type as secret_type, secret FROM aead_secret");
        let secrets: Vec<AeadSecretRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            secrets
                .iter()
                .map(|m| m.secret_type.clone())
                .collect::<Vec<String>>(),
            vec!["secret_1", "secret_2"]
        );
        Ok(())
    }

    async fn insert_authority_enrollment_tokens(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                r#"INSERT INTO authority_enrollment_token (one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes)
                   VALUES ('code_{index}', 'reference_{index}', 'issued_by_{index}', 10, 20, 20, $1)"#
            );
            let attributes = format!("attributes_{index}");
            let query = query(&q).bind(attributes.as_bytes());
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_authority_enrollment_tokens(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT one_time_code, reference, issued_by, created_at, expires_at, ttl_count, attributes FROM authority_enrollment_token");
        let tickets: Vec<AuthorityEnrollmentTicketRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            tickets
                .iter()
                .map(|m| m.one_time_code.clone())
                .collect::<Vec<String>>(),
            vec!["code_1", "code_2"]
        );
        Ok(())
    }

    async fn insert_credentials(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                r#"INSERT INTO credential (subject_identifier, issuer_identifier, scope, credential, expires_at, node_name)
                VALUES ('subject_identifier_{index}', 'issuer_identifier_{index}', 'scope_{index}', $1, 0, 'node_name_{index}')"#
            );
            query(&q)
                .bind(format!("'credential_{index}'").as_bytes())
                .execute(&*sqlite_database.pool)
                .await
                .void()?;
        }
        Ok(())
    }

    async fn check_credentials(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT subject_identifier, issuer_identifier, scope, credential, expires_at, node_name FROM credential");
        let credentials: Vec<CredentialRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            credentials
                .iter()
                .map(|m| m.subject_identifier.clone())
                .collect::<Vec<String>>(),
            vec!["subject_identifier_1", "subject_identifier_2"]
        );
        Ok(())
    }

    async fn insert_identities(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let query =
                query(r#"INSERT INTO identity (identifier, change_history) VALUES ($1, $2)"#)
                    .bind(format!("identity_{index}"))
                    .bind(format!("change_history_{index}"));
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_identities(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT identifier, change_history FROM identity");
        let identities: Vec<IdentityRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            identities
                .iter()
                .map(|m| m.identifier.clone())
                .collect::<Vec<String>>(),
            // The controller identity is hard-coded in the database.
            vec![
                "I84502ce0d9a0a91bae29026b84e19be69fb4203a6bdd1424c85a43c812772a00",
                "identity_1",
                "identity_2"
            ]
        );
        Ok(())
    }

    async fn insert_identity_attributes(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                r#"INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name)
                         VALUES ('identity_{index}', $1, 0, 10, 'attested_by_{index}', 'node_name_{index}')"#
            );
            query(&q)
                .bind(format!("attributes_{index}").as_bytes())
                .execute(&*sqlite_database.pool)
                .await
                .void()?;
        }
        Ok(())
    }

    async fn check_identity_attributes(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT identifier, attributes, added, expires, attested_by, node_name FROM identity_attributes");
        let attributes: Vec<IdentityAttributesRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            attributes
                .iter()
                .map(|m| m.identifier.clone())
                .collect::<Vec<String>>(),
            // The controller identity is hard-coded in the database.
            vec!["identity_1", "identity_2"]
        );
        Ok(())
    }

    async fn insert_members(sqlite_database: SqlxDatabase) -> Result<()> {
        let mut attributes = BTreeMap::new();
        attributes.insert("key", "value");
        for index in &["1", "2"] {
            let query = query(r#"INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes, authority_id)
                 VALUES ($1, $2, $3, $4, $5, $6)"#)
                .bind(format!("member_{index}"))
                .bind(format!("issuer_{index}"))
                .bind(0)
                .bind(true)
                .bind(ockam_core::cbor_encode_preallocate(attributes.clone())?)
                .bind("authority");
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_members(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT identifier, added_by, added_at, is_pre_trusted, attributes, authority_id FROM authority_member");
        let members: Vec<AuthorityMemberRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            members
                .iter()
                .map(|m| m.identifier.clone())
                .collect::<Vec<String>>(),
            vec!["member_1", "member_2"]
        );
        Ok(())
    }

    async fn insert_named_identities(sqlite_database: SqlxDatabase) -> Result<()> {
        for name in &["authority", "ockam-opentelemetry-outlet"] {
            let query =
                query(r#"INSERT INTO named_identity (identifier, name, vault_name, is_default) VALUES ($1, $2, $3, $4)"#)
                    .bind(format!("identity_{name}"))
                    .bind(name)
                    .bind("default")
                    .bind(false);
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        for index in &["1", "2"] {
            let query =
                query(r#"INSERT INTO named_identity (identifier, name, vault_name, is_default) VALUES ($1, $2, $3, $4)"#)
                    .bind(format!("identity_{index}"))
                    .bind(format!("name_{index}"))
                    .bind("default")
                    .bind(false);
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_named_identities(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT identifier, name, vault_name, is_default FROM named_identity");
        let identities: Vec<NamedIdentityRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        // Note that the authority and ockam-opentelemetry-outlet identities have not been exported.
        assert_eq!(
            identities
                .iter()
                .map(|m| (
                    m.identifier.clone(),
                    m.vault_name.clone(),
                    m.is_default.to_bool()
                ))
                .collect::<Vec<(String, String, bool)>>(),
            vec![
                ("identity_1".to_string(), "default".to_string(), false),
                ("identity_2".to_string(), "default".to_string(), false)
            ]
        );
        Ok(())
    }

    async fn insert_purpose_keys(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                r#"INSERT INTO purpose_key (identifier, purpose, purpose_key_attestation)
                VALUES ('identifier_{index}', 'purpose_{index}', $1)"#
            );
            query(&q)
                .bind(format!("purpose_key_attestation_{index}").as_bytes())
                .execute(&*sqlite_database.pool)
                .await
                .void()?;
        }
        Ok(())
    }

    async fn check_purpose_keys(postgres_database: SqlxDatabase) -> Result<()> {
        let query =
            query_as("SELECT identifier, purpose, purpose_key_attestation FROM purpose_key");
        let keys: Vec<PurposeKeyRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            keys.iter()
                .map(|m| m.identifier.clone())
                .collect::<Vec<String>>(),
            vec!["identifier_1", "identifier_2"]
        );
        Ok(())
    }

    async fn insert_signing_secrets(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = format!(
                "INSERT INTO signing_secret (handle, secret_type, secret) VALUES ($1, 'secret_{index}', $2)"
            );
            let handle = format!("handle_{index}");
            let secret = format!("secret_{index}");
            let query = query(&q).bind(handle.as_bytes()).bind(secret.as_bytes());
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_signing_secrets(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT handle, secret_type, secret FROM signing_secret");
        let secrets: Vec<SigningSecretRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            secrets
                .iter()
                .map(|m| m.secret_type.clone())
                .collect::<Vec<String>>(),
            vec!["secret_1", "secret_2"]
        );
        Ok(())
    }

    async fn insert_x25519_secrets(sqlite_database: SqlxDatabase) -> Result<()> {
        for index in &["1", "2"] {
            let q = "INSERT INTO x25519_secret (handle, secret) VALUES ($1, $2)";
            let handle = format!("handle_{index}");
            let secret = format!("secret_{index}");
            let query = query(q).bind(handle.as_bytes()).bind(secret.as_bytes());
            query.execute(&*sqlite_database.pool).await.void()?;
        }
        Ok(())
    }

    async fn check_x25519_secrets(postgres_database: SqlxDatabase) -> Result<()> {
        let query = query_as("SELECT handle, secret FROM x25519_secret");
        let secrets: Vec<X25519SecretRow> = query
            .fetch_all(&*postgres_database.pool)
            .await
            .into_core()?;

        assert_eq!(
            secrets
                .iter()
                .map(|m| m.handle.clone())
                .collect::<Vec<Vec<u8>>>(),
            vec![
                "handle_1".as_bytes().to_vec(),
                "handle_2".as_bytes().to_vec()
            ]
        );
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct AeadSecretRow {
    handle: Vec<u8>,
    secret_type: String,
    secret: Vec<u8>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct AuthorityEnrollmentTicketRow {
    one_time_code: String,
    reference: Option<String>,
    issued_by: String,
    created_at: i64,
    expires_at: i64,
    ttl_count: i64,
    attributes: Vec<u8>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct AuthorityMemberRow {
    identifier: String,
    added_by: String,
    added_at: i64,
    is_pre_trusted: Boolean,
    attributes: Vec<u8>,
    authority_id: String,
}

#[derive(sqlx::FromRow)]
pub(crate) struct CredentialRow {
    subject_identifier: String,
    issuer_identifier: String,
    scope: String,
    credential: Vec<u8>,
    expires_at: i64,
    node_name: String,
}

#[derive(sqlx::FromRow)]
pub(crate) struct IdentityRow {
    identifier: String,
    change_history: String,
}

#[derive(sqlx::FromRow)]
pub(crate) struct IdentityAttributesRow {
    identifier: String,
    attributes: Vec<u8>,
    added: i64,
    expires: i64,
    attested_by: String,
    node_name: String,
}

/// Clippy warns about dead code here but it shouldn't
#[allow(dead_code)]
#[derive(sqlx::FromRow)]
pub(crate) struct NamedIdentityRow {
    identifier: String,
    name: String,
    vault_name: String,
    is_default: Boolean,
}

#[derive(sqlx::FromRow)]
pub(crate) struct PurposeKeyRow {
    identifier: String,
    purpose: String,
    purpose_key_attestation: Vec<u8>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct SigningSecretRow {
    handle: Vec<u8>,
    secret_type: String,
    secret: Vec<u8>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct X25519SecretRow {
    handle: Vec<u8>,
    secret: Vec<u8>,
}
