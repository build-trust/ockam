use core::fmt::{Debug, Formatter};
use core::str::FromStr;
use std::future::Future;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ockam_core::errcode::{Kind, Origin};
use sqlx::any::{install_default_drivers, AnyConnectOptions};
use sqlx::pool::PoolOptions;
use sqlx::{Any, ConnectOptions, Pool};
use sqlx_core::any::AnyConnection;
use sqlx_core::executor::Executor;
use sqlx_core::row::Row;
use tempfile::NamedTempFile;
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;
use tracing::log::LevelFilter;

use crate::database::database_configuration::DatabaseConfiguration;
use crate::database::migrations::application_migration_set::ApplicationMigrationSet;
use crate::database::migrations::node_migration_set::NodeMigrationSet;
use crate::database::migrations::MigrationSet;
use crate::database::{DatabaseType, MigrationStatus};
use ockam_core::compat::rand::random_string;
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
    pub pool: Arc<Pool<Any>>,
    /// Configuration of the database
    pub configuration: DatabaseConfiguration,
}

impl Debug for SqlxDatabase {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("database options {:?}", self.pool.connect_options()).as_str())
    }
}

impl Deref for SqlxDatabase {
    type Target = Pool<Any>;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl SqlxDatabase {
    /// Constructor for a database
    pub async fn create(configuration: &DatabaseConfiguration) -> Result<Self> {
        Self::create_impl(
            configuration,
            Some(NodeMigrationSet::new(configuration.database_type())),
        )
        .await
    }

    /// Constructor for an application database
    pub async fn create_application_database(
        configuration: &DatabaseConfiguration,
    ) -> Result<Self> {
        Self::create_impl(
            configuration,
            Some(ApplicationMigrationSet::new(configuration.database_type())),
        )
        .await
    }

    /// Constructor for a sqlite database
    pub async fn create_sqlite(path: impl AsRef<Path>) -> Result<Self> {
        Self::create(&DatabaseConfiguration::sqlite(path)).await
    }

    /// Constructor for a sqlite database with no migrations
    pub async fn create_sqlite_no_migration(path: impl AsRef<Path>) -> Result<Self> {
        Self::create_no_migration(&DatabaseConfiguration::sqlite(path)).await
    }

    /// Constructor for a sqlite application database
    pub async fn create_application_sqlite(path: impl AsRef<Path>) -> Result<Self> {
        Self::create_application_database(&DatabaseConfiguration::sqlite(path)).await
    }

    /// Constructor for a postgres database that doesn't apply migrations
    pub async fn create_postgres_no_migration(legacy_sqlite_path: Option<PathBuf>) -> Result<Self> {
        match DatabaseConfiguration::postgres_with_legacy_sqlite_path(legacy_sqlite_path)? {
            Some(configuration) => Self::create_no_migration(&configuration).await,
            None => Err(Error::new(Origin::Core, Kind::NotFound, "There is no postgres database configuration, or it is incomplete. Please run ockam environment to check the database environment variables".to_string())),
        }
    }

    /// Constructor for a local postgres database with no data
    pub async fn create_new_postgres() -> Result<Self> {
        match DatabaseConfiguration::postgres()? {
            Some(configuration) => {
                let db = Self::create_no_migration(&configuration).await?;
                db.drop_all_postgres_tables().await?;
                SqlxDatabase::create(&configuration).await
            },
            None => Err(Error::new(Origin::Core, Kind::NotFound, "There is no postgres database configuration, or it is incomplete. Please run ockam environment to check the database environment variables".to_string())),
        }
    }

    /// Constructor for a local application postgres database with no data
    pub async fn create_new_application_postgres() -> Result<Self> {
        match DatabaseConfiguration::postgres()? {
            Some(configuration) => {
                let db = Self::create_application_no_migration(&configuration).await?;
                db.drop_all_postgres_tables().await?;
                SqlxDatabase::create_application_database(&configuration).await
            },
            None => Err(Error::new(Origin::Core, Kind::NotFound, "There is no postgres database configuration, or it is incomplete. Please run ockam environment to check the database environment variables".to_string())),
        }
    }

    /// Constructor for a database persisted on disk, with a specific schema / migration
    pub async fn create_with_migration(
        configuration: &DatabaseConfiguration,
        migration_set: impl MigrationSet,
    ) -> Result<Self> {
        Self::create_impl(configuration, Some(migration_set)).await
    }

    /// Constructor for a database persisted on disk without migration
    pub async fn create_no_migration(configuration: &DatabaseConfiguration) -> Result<Self> {
        Self::create_impl(configuration, None::<NodeMigrationSet>).await
    }

    /// Constructor for an application database persisted on disk without migration
    pub async fn create_application_no_migration(
        configuration: &DatabaseConfiguration,
    ) -> Result<Self> {
        Self::create_impl(configuration, None::<ApplicationMigrationSet>).await
    }

    async fn create_impl(
        configuration: &DatabaseConfiguration,
        migration_set: Option<impl MigrationSet>,
    ) -> Result<Self> {
        configuration.create_directory_if_necessary()?;

        // creating a new database might be failing a few times
        // if the files are currently being held by another pod which is shutting down.
        // In that case, we retry a few times, between 1 and 10 seconds.
        let retry_strategy = FixedInterval::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(10); // limit to 10 retries

        // migrate the database using exclusive locking only when operating with files
        let database = if configuration.database_type() == DatabaseType::Sqlite
            && configuration.path().is_some()
        {
            if let Some(migration_set) = migration_set {
                // To avoid any issues with the database being locked for more than necessary,
                // we open the database, run the migrations and close it.
                // (Changing the locking_mode back to NORMAL is not enough to release the lock)

                // We also request a single connection pool to avoid any issues with multiple
                // connections to a locked database.
                let migration_config = configuration.single_connection();

                let database = Retry::spawn(retry_strategy.clone(), || async {
                    match Self::create_at(&migration_config).await {
                        Ok(db) => Ok(db),
                        Err(e) => {
                            println!("{e:?}");
                            Err(e)
                        }
                    }
                })
                .await?;

                let migrator = migration_set.create_migrator()?;
                let status = migrator.migrate(&database.pool).await?;
                database.close().await;
                match status {
                    MigrationStatus::UpToDate(_) => (),
                    MigrationStatus::Todo(_, _) => (),
                    MigrationStatus::Failed(version, reason) => Err(Error::new(
                        Origin::Node,
                        Kind::Conflict,
                        format!(
                            "Sql migration previously failed for version {}. Reason: {}",
                            version, reason
                        ),
                    ))?,
                }
            };

            // re-create the connection pool with the correct configuration
            Retry::spawn(retry_strategy, || async {
                match Self::create_at(configuration).await {
                    Ok(db) => Ok(db),
                    Err(e) => {
                        println!("{e:?}");
                        Err(e)
                    }
                }
            })
            .await?
        } else {
            let database = Retry::spawn(retry_strategy, || async {
                match Self::create_at(configuration).await {
                    Ok(db) => Ok(db),
                    Err(e) => {
                        println!("{e:?}");
                        Err(e)
                    }
                }
            })
            .await?;

            // Only run the postgres migrations if the database has never been created.
            // This is mostly for tests. In production the database schema must be created separately
            // during the first deployment.
            let migrate_database = if configuration.database_type() == DatabaseType::Postgres {
                let database_schema_already_created: bool = sqlx::query("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'identity')")
                    .fetch_one(&*database.pool)
                    .await.into_core()?.get(0);
                !database_schema_already_created
            } else {
                true
            };

            if migrate_database {
                if let Some(migration_set) = migration_set {
                    let migrator = migration_set.create_migrator()?;
                    migrator.migrate(&database.pool).await?;
                }
            }

            database
        };

        Ok(database)
    }

    /// Create a nodes database in memory
    ///   => this database is deleted on an `ockam reset` command! (contrary to the application database below)
    pub async fn in_memory(usage: &str) -> Result<Self> {
        Self::in_memory_with_migration(usage, NodeMigrationSet::new(DatabaseType::Sqlite)).await
    }

    /// Create an application database in memory
    /// The application database which contains the application configurations
    ///   => this database is NOT deleted on an `ockam reset` command!
    pub async fn application_in_memory(usage: &str) -> Result<Self> {
        Self::in_memory_with_migration(usage, ApplicationMigrationSet::new(DatabaseType::Sqlite))
            .await
    }

    /// Create an in-memory database with a specific migration
    pub async fn in_memory_with_migration(
        usage: &str,
        migration_set: impl MigrationSet,
    ) -> Result<Self> {
        debug!("create an in memory database for {usage}");
        let configuration = DatabaseConfiguration::sqlite_in_memory();
        let pool = Self::create_in_memory_connection_pool().await?;
        let migrator = migration_set.create_migrator()?;
        migrator.migrate(&pool).await?;
        // FIXME: We should be careful if we run multiple nodes in one process
        let db = SqlxDatabase {
            pool: Arc::new(pool),
            configuration,
        };
        Ok(db)
    }

    /// Return true if the database implementation might lock (which is the case for Sqlite on disk)
    /// and the database user needs to retry several times.
    pub fn needs_retry(&self) -> bool {
        matches!(
            self.configuration,
            DatabaseConfiguration::SqlitePersistent { .. }
        )
    }

    async fn create_at(configuration: &DatabaseConfiguration) -> Result<Self> {
        // Creates database file if it doesn't exist
        let pool = Self::create_connection_pool(configuration).await?;
        Ok(SqlxDatabase {
            pool: Arc::new(pool),
            configuration: configuration.clone(),
        })
    }

    pub(crate) async fn create_connection_pool(
        configuration: &DatabaseConfiguration,
    ) -> Result<Pool<Any>> {
        install_default_drivers();
        let connection_string = configuration.connection_string();
        debug!("connecting to {connection_string}");
        let options = AnyConnectOptions::from_str(&connection_string)
            .map_err(Self::map_sql_err)?
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Trace, Duration::from_secs(1));

        // sqlx default is 10, 16 is closer to the typical number of threads spawn
        // by tokio within a node, but has no particular reason
        const MAX_POOL_SIZE: u32 = 16;

        let max_pool_size = match configuration {
            DatabaseConfiguration::SqlitePersistent {
                single_connection, ..
            }
            | DatabaseConfiguration::SqliteInMemory { single_connection } => {
                if *single_connection {
                    1
                } else {
                    MAX_POOL_SIZE
                }
            }
            _ => MAX_POOL_SIZE,
        };

        let pool_options = PoolOptions::new()
            .max_connections(max_pool_size)
            .min_connections(1);

        let pool_options = if configuration.database_type() == DatabaseType::Sqlite {
            // SQLite's configuration is specific for each connection, and needs to be set every time
            pool_options.after_connect(|connection: &mut AnyConnection, _metadata| {
                Box::pin(async move {
                    // Set configuration for SQLite, see https://www.sqlite.org/pragma.html
                    // synchronous = EXTRA - trade performance for durability and reliability
                    // locking_mode = NORMAL - it's important because WAL mode changes behavior
                    //                         if locking_mode is set to EXCLUSIVE *before* WAL is set
                    // busy_timeout = 10000 - wait for 10 seconds before failing a query due to exclusive lock
                    let _ = connection
                        .execute(
                            r#"
PRAGMA synchronous = EXTRA;
PRAGMA locking_mode = NORMAL;
PRAGMA busy_timeout = 10000;
                "#,
                        )
                        .await
                        .expect("Failed to set SQLite configuration");

                    Ok(())
                })
            })
        } else {
            pool_options
        };

        let pool = pool_options
            .connect_with(options)
            .await
            .map_err(Self::map_sql_err)?;

        Ok(pool)
    }

    /// Create a connection for a SQLite database
    pub async fn create_sqlite_single_connection_pool(path: impl AsRef<Path>) -> Result<Pool<Any>> {
        Self::create_connection_pool(&DatabaseConfiguration::sqlite(path).single_connection()).await
    }

    pub(crate) async fn create_in_memory_connection_pool() -> Result<Pool<Any>> {
        install_default_drivers();
        // SQLite in-memory DB get wiped if there is no connection to it.
        // The below setting tries to ensure there is always an open connection
        let file_name = random_string();
        let options = AnyConnectOptions::from_str(
            format!("sqlite:file:{file_name}?mode=memory&cache=shared").as_str(),
        )
        .map_err(Self::map_sql_err)?
        .log_statements(LevelFilter::Trace)
        .log_slow_statements(LevelFilter::Trace, Duration::from_secs(1));
        let pool_options = PoolOptions::new().idle_timeout(None).max_lifetime(None);

        let pool = pool_options
            .connect_with(options)
            .await
            .map_err(Self::map_sql_err)?;
        Ok(pool)
    }

    /// Path to the db file if there is one
    pub fn path(&self) -> Option<PathBuf> {
        self.configuration.path()
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

    /// Drop all the postgres database tables
    pub async fn drop_all_postgres_tables(&self) -> Result<()> {
        self.clean_postgres_node_tables(Clean::Drop, None).await
    }

    /// Truncate all the postgres database tables
    pub async fn truncate_all_postgres_tables(&self) -> Result<()> {
        self.clean_postgres_node_tables(Clean::Truncate, None).await
    }

    /// Drop all the database tables _except_ for the journey tables
    pub async fn drop_postgres_node_tables(&self) -> Result<()> {
        self.clean_postgres_node_tables(Clean::Drop, Some("AND tablename NOT LIKE '%journey%'"))
            .await
    }

    /// Truncate all the database tables _except_ for the journey tables
    pub async fn truncate_postgres_node_tables(&self) -> Result<()> {
        self.clean_postgres_node_tables(Clean::Truncate, Some("AND tablename NOT LIKE '%journey%'"))
            .await
    }

    /// Truncate all the database tables _except_ for the journey tables
    async fn clean_postgres_node_tables(&self, clean: Clean, filter: Option<&str>) -> Result<()> {
        match self.configuration.database_type() {
            DatabaseType::Sqlite => Ok(()),
            DatabaseType::Postgres => {
                sqlx::query(
                    format!(r#"DO $$
                   DECLARE
                       r RECORD;
                   BEGIN
                       FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public' {}) LOOP
                           EXECUTE '{} TABLE ' || quote_ident(r.tablename) || ' CASCADE';
                       END LOOP;
                   END $$;"#, filter.unwrap_or(""), clean.as_str(),
                    ).as_str())
                    .execute(&*self.pool)
                    .await
                    .void()
            }
        }
    }
}

enum Clean {
    Drop,
    Truncate,
}

impl Clean {
    fn as_str(&self) -> &str {
        match self {
            Clean::Drop => "DROP",
            Clean::Truncate => "TRUNCATE",
        }
    }
}

/// This function can be used to run some test code with the 2 SQLite databases implementations
pub async fn with_sqlite_dbs<F, Fut>(f: F) -> Result<()>
where
    F: Fn(SqlxDatabase) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let db = SqlxDatabase::in_memory("test").await?;
    rethrow("SQLite in memory", f(db)).await?;

    let db_file = NamedTempFile::new().unwrap();
    let db = SqlxDatabase::create_sqlite(db_file.path()).await?;
    rethrow("SQLite on disk", f(db)).await?;
    Ok(())
}

/// This function can be used to run some test code with the 3 different databases implementations
pub async fn with_dbs<F, Fut>(f: F) -> Result<()>
where
    F: Fn(SqlxDatabase) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let db = SqlxDatabase::in_memory("test").await?;
    rethrow("SQLite in memory", f(db)).await?;

    let db_file = NamedTempFile::new().unwrap();
    let db = SqlxDatabase::create_sqlite(db_file.path()).await?;
    rethrow("SQLite on disk", f(db)).await?;

    // only run the postgres tests if the OCKAM_DATABASE_CONNECTION_URL environment variables is set
    with_postgres(f).await?;
    Ok(())
}

/// This function can be used to run some test code with a postgres database
pub async fn with_postgres<F, Fut>(f: F) -> Result<()>
where
    F: Fn(SqlxDatabase) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    // only run the postgres tests if the OCKAM_DATABASE_CONNECTION_URL environment variables is set
    if let Ok(db) = SqlxDatabase::create_new_postgres().await {
        db.truncate_all_postgres_tables().await?;
        rethrow("Postgres local", f(db.clone())).await?;
    };
    Ok(())
}

/// This function can be used to avoid running a test if the postgres database is used.
pub async fn skip_if_postgres<F, Fut, R>(f: F) -> std::result::Result<(), R>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::result::Result<(), R>> + Send + 'static,
    R: From<Error>,
{
    // only run the postgres tests if the OCKAM_DATABASE_CONNECTION_URL environment variable is not set
    if DatabaseConfiguration::postgres()?.is_none() {
        f().await?
    };
    Ok(())
}

/// This function can be used to run some test code with the 3 different databases implementations
/// of the application database
pub async fn with_application_dbs<F, Fut>(f: F) -> Result<()>
where
    F: Fn(SqlxDatabase) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let db = SqlxDatabase::application_in_memory("test").await?;
    rethrow("SQLite in memory", f(db)).await?;

    let db_file = NamedTempFile::new().unwrap();
    let db = SqlxDatabase::create_application_sqlite(db_file.path()).await?;
    rethrow("SQLite on disk", f(db)).await?;

    // only run the postgres tests if the OCKAM_DATABASE_CONNECTION_URL environment variable is set
    if let Ok(db) = SqlxDatabase::create_new_application_postgres().await {
        rethrow("Postgres local", f(db.clone())).await?;
        db.drop_all_postgres_tables().await?;
    }
    Ok(())
}

/// Specify which database was used to run a test
async fn rethrow<Fut>(database_type: &str, f: Fut) -> Result<()>
where
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    f.await.map_err(|e| {
        Error::new(
            Origin::Core,
            Kind::Invalid,
            format!("{database_type}: {e:?}"),
        )
    })
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

impl<T> FromSqlxError<T> for core::result::Result<T, sqlx::migrate::MigrateError> {
    #[track_caller]
    fn into_core(self) -> Result<T> {
        match self {
            Ok(r) => Ok(r),
            Err(err) => Err(Error::new(
                Origin::Application,
                Kind::Io,
                format!("migration error {err}"),
            )),
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

/// Create a temporary database file that won't be cleaned-up automatically
pub fn create_temp_db_file() -> Result<PathBuf> {
    let (_, path) = NamedTempFile::new()
        .map_err(|e| Error::new(Origin::Core, Kind::Io, format!("{e:?}")))?
        .keep()
        .map_err(|e| Error::new(Origin::Core, Kind::Io, format!("{e:?}")))?;
    Ok(path)
}

#[cfg(test)]
#[allow(missing_docs)]
pub mod tests {
    use super::*;
    use crate::database::Boolean;
    use sqlx::any::AnyQueryResult;
    use sqlx::FromRow;

    /// This is a sanity check to test that the database can be created with a file path
    /// and that migrations are running ok, at least for one table
    #[tokio::test]
    async fn test_create_sqlite_database() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDatabase::create_sqlite(db_file.path()).await?;

        let inserted = insert_identity(&db).await.unwrap();

        assert_eq!(inserted.rows_affected(), 1);
        Ok(())
    }

    /// This is a sanity check to test that we can use Postgres as a database
    #[tokio::test]
    async fn test_create_postgres_database() -> Result<()> {
        if let Some(configuration) = DatabaseConfiguration::postgres()? {
            let db = SqlxDatabase::create_no_migration(&configuration).await?;
            db.drop_all_postgres_tables().await?;

            let db = SqlxDatabase::create(&configuration).await?;
            let inserted = insert_identity(&db).await.unwrap();
            assert_eq!(inserted.rows_affected(), 1);
        }
        Ok(())
    }

    /// This test checks that we can run a query and return an entity
    #[tokio::test]
    async fn test_query() -> Result<()> {
        with_dbs(|db| async move {
            insert_identity(&db).await.unwrap();

            // successful query
            let result: Option<IdentifierRow> =
                sqlx::query_as("SELECT identifier, name, vault_name, is_default FROM named_identity WHERE identifier = $1")
                    .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
                    .fetch_optional(&*db.pool)
                    .await
                    .unwrap();
            assert_eq!(
                result,
                Some(IdentifierRow {
                    identifier: "Ifa804b7fca12a19eed206ae180b5b576860ae651".into(),
                    name: "identity-1".to_string(),
                    vault_name: "vault-1".to_string(),
                    // This line tests the proper deserialization of a Boolean
                    // in SQLite where a Boolean maps to an INTEGER
                    is_default: Boolean::new(true),
                })
            );

            // failed query
            let result: Option<IdentifierRow> =
                sqlx::query_as("SELECT identifier FROM named_identity WHERE identifier = $1")
                    .bind("x")
                    .fetch_optional(&*db.pool)
                    .await
                    .unwrap();
            assert_eq!(result, None);
            Ok(())
        }).await
    }

    #[tokio::test]
    async fn test_create_pool_with_relative_and_absolute_paths() -> Result<()> {
        install_default_drivers();
        let relative = Path::new("relative");
        let connection_string = DatabaseConfiguration::sqlite(relative).connection_string();
        let options =
            AnyConnectOptions::from_str(&connection_string).map_err(SqlxDatabase::map_sql_err)?;

        let pool = Pool::<Any>::connect_with(options)
            .await
            .map_err(SqlxDatabase::map_sql_err);
        assert!(pool.is_ok());

        let absolute = std::fs::canonicalize(relative).unwrap();
        let connection_string = DatabaseConfiguration::sqlite(&absolute).connection_string();
        let options =
            AnyConnectOptions::from_str(&connection_string).map_err(SqlxDatabase::map_sql_err)?;

        let pool = Pool::<Any>::connect_with(options)
            .await
            .map_err(SqlxDatabase::map_sql_err);
        assert!(pool.is_ok());

        let _ = std::fs::remove_file(absolute);

        Ok(())
    }

    /// HELPERS
    async fn insert_identity(db: &SqlxDatabase) -> Result<AnyQueryResult> {
        sqlx::query("INSERT INTO named_identity (identifier, name, vault_name, is_default) VALUES ($1, $2, $3, $4)")
            .bind("Ifa804b7fca12a19eed206ae180b5b576860ae651")
            .bind("identity-1")
            .bind("vault-1")
            .bind(true)
            .execute(&*db.pool)
            .await
            .into_core()
    }

    #[derive(FromRow, PartialEq, Eq, Debug)]
    struct IdentifierRow {
        identifier: String,
        name: String,
        vault_name: String,
        is_default: Boolean,
    }
}
