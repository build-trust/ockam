use core::fmt::{Debug, Formatter};
use sqlx::pool::PoolOptions;
use sqlx::sqlite::SqliteConnectOptions;
use std::ops::Deref;
use std::path::Path;

use ockam_core::errcode::{Kind, Origin};
use sqlx::{ConnectOptions, SqlitePool};
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;
use tracing::log::LevelFilter;

use crate::database::application_migration::ApplicationMigration;
use crate::database::migrations::sqlx_migration::{NodesMigration, SqlxMigration};
use ockam_core::compat::sync::Arc;
use ockam_core::{Error, Result};

/// The SqlxDatabase struct is used to create a database:
///   - at a given path
///   - with a given schema / or migrations applied to an existing schema
///
/// We use sqlx as our primary interface for interacting with the database
/// The database driver is currently Sqlite
#[derive(Clone)]
pub struct SqlxDatabase {
    /// Pool of connections to the database
    pub pool: Arc<SqlitePool>,
    /// Node name to isolate data between nodes where needed
    pub node_name: Option<String>,
}

impl Debug for SqlxDatabase {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("database options {:?}", self.pool.connect_options()).as_str())
    }
}

impl Deref for SqlxDatabase {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl SqlxDatabase {
    /// Constructor for a database persisted on disk
    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        Self::create_impl(path, Some(NodesMigration), None).await
    }

    /// Constructor for a database persisted on disk, with a specific schema / migration
    pub async fn create_with_migration(
        path: impl AsRef<Path>,
        migration: impl SqlxMigration,
    ) -> Result<Self> {
        Self::create_impl(path, Some(migration), None).await
    }

    /// Constructor for a database persisted on disk without migration
    pub async fn create_no_migration(path: impl AsRef<Path>) -> Result<Self> {
        Self::create_impl(path, None::<NodesMigration>, None).await
    }

    /// Constructor for a database persisted on disk, passing a node name to isolate data between nodes where needed
    pub async fn create_with_node_name(path: impl AsRef<Path>, node_name: &str) -> Result<Self> {
        Self::create_impl(path, Some(NodesMigration), Some(node_name.to_string())).await
    }

    async fn create_impl(
        path: impl AsRef<Path>,
        migration: Option<impl SqlxMigration>,
        node_name: Option<String>,
    ) -> Result<Self> {
        path.as_ref()
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .map_err(|e| Error::new(Origin::Api, Kind::Io, e.to_string()))?;

        // creating a new database might be failing a few times
        // if the files are currently being held by another pod which is shutting down.
        // In that case we retry a few times, between 1 and 10 seconds.
        let retry_strategy = FixedInterval::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(10); // limit to 10 retries

        let db = Retry::spawn(retry_strategy, || async {
            Self::create_at(path.as_ref(), node_name.clone()).await
        })
        .await?;

        if let Some(migration) = migration {
            migration.migrate(&db.pool).await?;
        }

        Ok(db)
    }

    /// Create a nodes database in memory
    ///   => this database is deleted on an `ockam reset` command! (contrary to the application database below)
    pub async fn in_memory(usage: &str) -> Result<Self> {
        Self::in_memory_with_migration(usage, NodesMigration).await
    }

    /// Create an application database in memory
    /// The application database which contains the application configurations
    ///   => this database is NOT deleted on an `ockam reset` command!
    pub async fn application_in_memory(usage: &str) -> Result<Self> {
        Self::in_memory_with_migration(usage, ApplicationMigration).await
    }

    /// Create an in-memory database with a specific migration
    pub async fn in_memory_with_migration(
        usage: &str,
        migration: impl SqlxMigration,
    ) -> Result<Self> {
        debug!("create an in memory database for {usage}");
        let pool = Self::create_in_memory_connection_pool().await?;
        migration.migrate(&pool).await?;
        // FIXME: We should be careful if we run multiple nodes in one process
        let db = SqlxDatabase {
            pool: Arc::new(pool),
            node_name: Some("in_memory".to_string()),
        };
        Ok(db)
    }

    async fn create_at(path: &Path, node_name: Option<String>) -> Result<Self> {
        // Creates database file if it doesn't exist
        let pool = Self::create_connection_pool(path).await?;
        Ok(SqlxDatabase {
            pool: Arc::new(pool),
            node_name,
        })
    }

    pub(crate) async fn create_connection_pool(path: &Path) -> Result<SqlitePool> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .log_statements(LevelFilter::Debug);
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(Self::map_sql_err)?;
        Ok(pool)
    }

    pub(crate) async fn create_in_memory_connection_pool() -> Result<SqlitePool> {
        // SQLite in-memory DB get wiped if there is no connection to it.
        // The below setting tries to ensure there is always an open connection
        let pool_options = PoolOptions::new().idle_timeout(None).max_lifetime(None);

        let pool = pool_options
            .connect("sqlite::memory:")
            .await
            .map_err(Self::map_sql_err)?;
        Ok(pool)
    }

    /// Return the node name
    pub fn node_name(&self) -> Result<String> {
        self.node_name.clone().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Internal,
                "The node name is not set",
            )
        })
    }

    /// Map a sqlx error into an ockam error
    #[track_caller]
    pub fn map_sql_err(err: sqlx::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }

    /// Map a minicbor decode error into an ockam error
    #[track_caller]
    pub fn map_decode_err(err: minicbor::decode::Error) -> Error {
        Error::new(Origin::Application, Kind::Io, err)
    }
}

/// This trait provides some syntax for transforming sqlx errors into ockam errors
pub trait FromSqlxError<T> {
    /// Make an ockam core Error
    fn into_core(self) -> Result<T>;
}

impl<T> FromSqlxError<T> for core::result::Result<T, sqlx::error::Error> {
    #[track_caller]
    fn into_core(self) -> Result<T> {
        match self {
            Ok(r) => Ok(r),
            Err(err) => {
                let err = Error::new(Origin::Api, Kind::Internal, err.to_string());
                Err(err)
            }
        }
    }
}

/// This trait provides some syntax to shorten queries execution returning ()
pub trait ToVoid<T> {
    /// Return a () value
    fn void(self) -> Result<()>;
}

impl<T> ToVoid<T> for core::result::Result<T, sqlx::error::Error> {
    #[track_caller]
    fn void(self) -> Result<()> {
        self.map(|_| ()).into_core()
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqliteQueryResult;
    use sqlx::FromRow;
    use tempfile::NamedTempFile;

    use crate::database::ToSqlxType;

    use super::*;

    /// This is a sanity check to test that the database can be created with a file path
    /// and that migrations are running ok, at least for one table
    #[tokio::test]
    async fn test_create_identity_table() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDatabase::create(db_file.path()).await?;

        let inserted = insert_identity(&db).await.unwrap();

        assert_eq!(inserted.rows_affected(), 1);
        Ok(())
    }

    /// This test checks that we can run a query and return an entity
    #[tokio::test]
    async fn test_query() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDatabase::create(db_file.path()).await?;

        insert_identity(&db).await.unwrap();

        // successful query
        let result: Option<IdentifierRow> =
            sqlx::query_as("SELECT identifier FROM identity WHERE identifier=?1")
                .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
                .fetch_optional(&*db.pool)
                .await
                .unwrap();
        assert_eq!(
            result,
            Some(IdentifierRow(
                "Ifa804b7fca12a19eed206ae180b5b576860ae651".into()
            ))
        );

        // failed query
        let result: Option<IdentifierRow> =
            sqlx::query_as("SELECT identifier FROM identity WHERE identifier=?1")
                .bind("x")
                .fetch_optional(&*db.pool)
                .await
                .unwrap();
        assert_eq!(result, None);
        Ok(())
    }

    /// HELPERS
    async fn insert_identity(db: &SqlxDatabase) -> Result<SqliteQueryResult> {
        sqlx::query("INSERT INTO identity VALUES (?1, ?2)")
            .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
            .bind("123".to_sql())
            .execute(&*db.pool)
            .await
            .into_core()
    }

    #[derive(FromRow, PartialEq, Eq, Debug)]
    struct IdentifierRow(String);
}
