use core::fmt::{Display, Formatter};
use ockam_core::compat::rand::random_string;
use ockam_core::env::{get_env, get_env_with_default};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use percent_encoding::NON_ALPHANUMERIC;
use serde_json::Value;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use tracing::log::LevelFilter;

/// Use an in-memory SQLite database
pub const OCKAM_SQLITE_IN_MEMORY: &str = "OCKAM_SQLITE_IN_MEMORY";
/// Database connection URL
pub const OCKAM_DATABASE_CONNECTION_URL: &str = "OCKAM_DATABASE_CONNECTION_URL";
/// Database instance as HOST:PORT/name
pub const OCKAM_DATABASE_INSTANCE: &str = "OCKAM_DATABASE_INSTANCE";
/// Database user
pub const OCKAM_DATABASE_USER: &str = "OCKAM_DATABASE_USER";
/// Database password
pub const OCKAM_DATABASE_PASSWORD: &str = "OCKAM_DATABASE_PASSWORD";
/// Database user + password in the format {"username":"pgadmin", "password":"s3cr3t"}
pub const OCKAM_DATABASE_USERNAME_AND_PASSWORD: &str = "OCKAM_DATABASE_USERNAME_AND_PASSWORD";
/// Name of the database admin user
pub const OCKAM_DATABASE_ADMIN_USERNAME: &str = "OCKAM_DATABASE_ADMIN_USERNAME";
/// Log level for sql statements. Accepted values, see LevelVar. For example: trace, debug, info, warn, error
pub const OCKAM_SQL_LOG_LEVEL: &str = "OCKAM_SQL_LOG_LEVEL";

/// Configuration for the database.
/// We either use Sqlite or Postgres
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfiguration {
    mode: DatabaseConfigurationMode,
    statements_log_level: LevelFilter,
}

impl DatabaseConfiguration {
    /// Constructor
    pub fn new(mode: DatabaseConfigurationMode, statements_log_level: LevelFilter) -> Self {
        Self {
            mode,
            statements_log_level,
        }
    }

    /// Create with a mode and default log level
    pub fn create(mode: DatabaseConfigurationMode) -> Result<Self> {
        let statements_log_level = Self::get_env_statements_log_level()?;

        Ok(Self::new(mode, statements_log_level))
    }

    /// Mode
    pub fn mode(&self) -> &DatabaseConfigurationMode {
        &self.mode
    }

    /// If can't have more than 1 connection in the pool
    pub fn is_single_connection(&self) -> bool {
        match self.mode() {
            DatabaseConfigurationMode::SqlitePersistent {
                single_connection, ..
            } => *single_connection,
            DatabaseConfigurationMode::SqliteInMemory { single_connection } => *single_connection,
            _ => false,
        }
    }

    /// If in-memory mode is used
    pub fn is_in_memory(&self) -> bool {
        matches!(
            self.mode(),
            DatabaseConfigurationMode::SqliteInMemory { .. }
        )
    }

    /// Statements log level
    pub fn statements_log_level(&self) -> LevelFilter {
        self.statements_log_level
    }
}

/// Configuration for the database.
/// We either use Sqlite or Postgres
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabaseConfigurationMode {
    /// Configuration for a SQLite database
    SqlitePersistent {
        /// Database file path if the database is stored on disk
        path: PathBuf,
        /// Set the connection pool size to 1, needed for the initial migration
        single_connection: bool,
    },
    /// Configuration for a SQLite database
    SqliteInMemory {
        /// Set the connection pool size to 1, needed for the initial migration
        single_connection: bool,
    },
    /// Configuration for a Postgres database
    Postgres {
        /// Connection URL of the form postgres://[{user}:{password}@]{host}:{port}/{database_name}
        connection_url: ConnectionUrl,
        /// Path to a SQLite database that needs to be migrated to the Postgres database.
        legacy_sqlite_path: Option<PathBuf>,
        /// Name of the database admin user
        admin_user: String,
    },
}

/// Type of database
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabaseType {
    /// Type for SQLite
    Sqlite,
    /// Type for Postgres
    Postgres,
}

/// User of the Postgres database
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseUser {
    /// Database user
    user_name: String,
    /// Database password
    password: String,
}

impl DatabaseUser {
    /// Create a new database user
    pub fn new(user_name: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            password: password.into(),
        }
    }
    /// Return the user name
    pub fn user_name(&self) -> String {
        self.user_name.clone()
    }
    /// Return the password
    pub fn password(&self) -> String {
        self.password.clone()
    }
}

impl DatabaseConfiguration {
    fn get_env_statements_log_level() -> Result<LevelFilter> {
        get_env_with_default(OCKAM_SQL_LOG_LEVEL, LevelFilter::Trace)
    }

    /// Create a postgres database configuration from an environment variable.
    pub fn postgres() -> Result<Option<DatabaseConfiguration>> {
        Self::postgres_with_legacy_sqlite_path(None)
    }

    /// Create a postgres database configuration from an environment variable.
    /// An optional legacy sqlite path can be provided to migrate the sqlite database to postgres.
    pub fn postgres_with_legacy_sqlite_path(
        sqlite_path: Option<PathBuf>,
    ) -> Result<Option<DatabaseConfiguration>> {
        if let Some(connection_string) = get_database_connection_url()? {
            let admin_user =
                get_env::<String>(OCKAM_DATABASE_ADMIN_USERNAME)?.unwrap_or("postgres".to_string());

            let mode = DatabaseConfigurationMode::Postgres {
                connection_url: parse_connection_string(&connection_string)?,
                legacy_sqlite_path: sqlite_path,
                admin_user,
            };

            let configuration = DatabaseConfiguration::create(mode)?;

            Ok(Some(configuration))
        } else {
            Ok(None)
        }
    }

    /// Create a local sqlite configuration
    pub fn sqlite(path: impl AsRef<Path>) -> Result<Self> {
        let mode = DatabaseConfigurationMode::SqlitePersistent {
            path: path.as_ref().to_path_buf(),
            single_connection: false,
        };

        Self::create(mode)
    }

    /// Create an in-memory sqlite configuration
    pub fn sqlite_in_memory() -> Result<Self> {
        let mode = DatabaseConfigurationMode::SqliteInMemory {
            single_connection: false,
        };

        Self::create(mode)
    }

    /// Create a single connection sqlite configuration
    pub fn single_connection(&self) -> Self {
        let mode = match self.mode() {
            DatabaseConfigurationMode::SqlitePersistent { path, .. } => {
                DatabaseConfigurationMode::SqlitePersistent {
                    path: path.clone(),
                    single_connection: true,
                }
            }
            DatabaseConfigurationMode::SqliteInMemory { .. } => {
                DatabaseConfigurationMode::SqliteInMemory {
                    single_connection: true,
                }
            }
            _ => self.mode.clone(),
        };

        Self::new(mode, self.statements_log_level)
    }

    /// Return the type of database that has been configured
    pub fn database_type(&self) -> DatabaseType {
        match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => DatabaseType::Sqlite,
            DatabaseConfigurationMode::SqlitePersistent { .. } => DatabaseType::Sqlite,
            DatabaseConfigurationMode::Postgres { .. } => DatabaseType::Postgres,
        }
    }

    /// Return the connection user if it is defined
    pub fn user(&self) -> Option<String> {
        match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => None,
            DatabaseConfigurationMode::SqlitePersistent { .. } => None,
            DatabaseConfigurationMode::Postgres { connection_url, .. } => {
                Some(connection_url.user())
            }
        }
    }

    /// Change the user if this is a Postgres configuration
    pub fn switch_to_user(&self, user: &str, password: &str) -> Self {
        let mode = match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => self.mode.clone(),
            DatabaseConfigurationMode::SqlitePersistent { .. } => self.mode.clone(),
            DatabaseConfigurationMode::Postgres {
                connection_url,
                legacy_sqlite_path,
                admin_user,
            } => DatabaseConfigurationMode::Postgres {
                connection_url: connection_url.switch_to_user(user, password),
                legacy_sqlite_path: legacy_sqlite_path.clone(),
                admin_user: admin_user.clone(),
            },
        };

        Self::new(mode, self.statements_log_level)
    }

    /// Return the connection user if it is defined
    pub fn is_admin_user(&self) -> bool {
        match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => true,
            DatabaseConfigurationMode::SqlitePersistent { .. } => true,
            DatabaseConfigurationMode::Postgres {
                admin_user,
                connection_url,
                ..
            } => connection_url.user() == *admin_user,
        }
    }

    /// Return the legacy sqlite path if any
    pub fn legacy_sqlite_path(&self) -> Option<PathBuf> {
        match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => None,
            DatabaseConfigurationMode::SqlitePersistent { .. } => None,
            DatabaseConfigurationMode::Postgres {
                legacy_sqlite_path, ..
            } => legacy_sqlite_path.clone(),
        }
    }

    /// Return the type of database that has been configured
    pub fn connection_string(&self) -> String {
        match self.mode() {
            DatabaseConfigurationMode::SqliteInMemory { .. } => {
                Self::create_sqlite_in_memory_connection_string()
            }
            DatabaseConfigurationMode::SqlitePersistent { path, .. } => {
                Self::create_sqlite_on_disk_connection_string(path)
            }
            DatabaseConfigurationMode::Postgres { connection_url, .. } => {
                connection_url.to_string()
            }
        }
    }

    /// Create a directory for the SQLite database file if necessary
    pub fn create_directory_if_necessary(&self) -> Result<()> {
        if let DatabaseConfigurationMode::SqlitePersistent { path, .. } = self.mode() {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    create_dir_all(parent)
                        .map_err(|e| Error::new(Origin::Api, Kind::Io, e.to_string()))?
                }
            }
        }
        Ok(())
    }

    /// Return true if the path for a SQLite database exists
    pub fn exists(&self) -> bool {
        self.path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Return the database path if the database is a SQLite file.
    pub fn path(&self) -> Option<PathBuf> {
        match self.mode() {
            DatabaseConfigurationMode::SqlitePersistent { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    fn create_sqlite_in_memory_connection_string() -> String {
        // SQLite in-memory DB get wiped if there is no connection to it.
        // The below setting tries to ensure there is always an open connection
        let file_name = random_string();
        format!("sqlite:file:{file_name}?mode=memory&cache=shared")
    }

    fn create_sqlite_on_disk_connection_string(path: &Path) -> String {
        let url_string = &path.to_string_lossy().to_string();
        format!("sqlite://{url_string}?mode=rwc")
    }
}

/// We can either get the connection string directly from the OCKAM_DATABASE_CONNECTION_URL environment variable,
/// or we can build it from other variables. Either from:
///
///  - The database instance name + user + password,
///  - Or the database instance name + user & password as a single JSON string.
///
/// This useful when:
///
/// - The password is rotated externally, by the AWS Secrets Manager service.
/// - The password needs to be url encoded.
///
fn get_database_connection_url() -> Result<Option<String>> {
    let connection_string = match get_env::<String>(OCKAM_DATABASE_CONNECTION_URL)? {
        Some(connection_string) => connection_string,
        None => {
            let (instance, user, password) = match (
                get_env::<String>(OCKAM_DATABASE_INSTANCE)?,
                get_env::<String>(OCKAM_DATABASE_USER)?,
                get_env::<String>(OCKAM_DATABASE_PASSWORD)?,
                get_env::<String>(OCKAM_DATABASE_USERNAME_AND_PASSWORD)?,
            ) {
                (Some(instance), Some(user), Some(password), None) => (instance, user, password),
                (Some(instance), None, None, Some(user_and_password)) => {
                    let parsed: Value = serde_json::from_str(&user_and_password).map_err(|_| {
                        Error::new(
                            Origin::Api,
                            Kind::Invalid,
                            format!("Expected a JSON object. Got: {user_and_password}"),
                        )
                    })?;
                    if let (Some(user), Some(password)) =
                        (parsed["username"].as_str(), parsed["password"].as_str())
                    {
                        (instance, user.to_string(), password.to_string())
                    } else {
                        return Err(Error::new(
                            Origin::Api,
                            Kind::Invalid,
                            format!(
                                "Expected the username and password as `{}`.
                            Got: {user_and_password}",
                                r#"{"username":"pgadmin", "password":"12345"}"#
                            ),
                        ));
                    }
                }
                _ => return Ok(None),
            };
            // A password can contain special characters, so we need to encode it.
            let url_encoded_password =
                percent_encoding::utf8_percent_encode(&password, NON_ALPHANUMERIC);
            format!("postgres://{user}:{url_encoded_password}@{instance}")
        }
    };
    parse_connection_string(&connection_string)?;
    Ok(Some(connection_string))
}

/// A connection URL for a Postgres database
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionUrl {
    user: String,
    password: String,
    host: String,
    port: u16,
    database_name: String,
}

impl Display for ConnectionUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database_name
        );
        f.write_str(&url)
    }
}

impl ConnectionUrl {
    /// Return the connection user
    pub fn user(&self) -> String {
        self.user.clone()
    }

    /// Set a different user and password on this connection
    pub fn switch_to_user(&self, user: &str, password: &str) -> Self {
        Self {
            user: user.to_string(),
            password: password.to_string(),
            ..self.clone()
        }
    }
}

/// Check the format of a database connection string as `postgres://{user}:{password}@{host}:{port}/{database_name}`
/// For now we only support postgres.
pub fn parse_connection_string(connection_string: &str) -> Result<ConnectionUrl> {
    if let Some(no_prefix) = connection_string.strip_prefix("postgres://") {
        let (user, password, host_port_db_name) = match no_prefix.split('@').collect::<Vec<_>>()[..]
        {
            [user_and_password, host_port_db_name] => {
                let user_and_password = &user_and_password.split(':').collect::<Vec<_>>()[..];
                match user_and_password {
                    [user, password] => (user.to_string(), password.to_string(), host_port_db_name),
                    _ => {return Err(Error::new(
                        Origin::Api,
                        Kind::Invalid,
                        "A database connection URL must specify the user and password as user:password".to_string(),
                    ))}
                }
            }
            _ => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    "A database connection URL can only have one @ separator to specify the user name and password".to_string(),
                ));
            }
        };
        let (host, port, database_name) = match host_port_db_name.split('/').collect::<Vec<_>>()[..] {
            [host_port, database_name] => {
                let host_port = &host_port.split(':').collect::<Vec<_>>()[..];
                match host_port {
                    [host, port] =>
                        if let Ok(p) = port.parse::<u16>() {
                          (host.to_string(), p, database_name.to_string())
                        } else {
                            return Err(Error::new(
                                Origin::Api,
                                Kind::Invalid,
                                "The database port must be a u16 value".to_string(),
                            ))
                        }

                    _ => {
                        return Err(Error::new(
                            Origin::Api,
                            Kind::Invalid,
                            "A database connection URL must have a host and a port specified as host:port".to_string(),
                        ))

                    }
                }
            }
            _ => return Err(Error::new(
                Origin::Api,
                Kind::Invalid,
                "A database connection URL must have a host, a port and a database name as host:port/database_name".to_string(),
            )),
        };
        Ok(ConnectionUrl {
            user,
            password,
            host,
            port,
            database_name,
        })
    } else {
        Err(Error::new(
            Origin::Api,
            Kind::Invalid,
            "A database connection must start with postgres://".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_make_connection_url_from_separate_env_variables() -> Result<()> {
        env::remove_var(OCKAM_DATABASE_CONNECTION_URL);
        env::set_var(OCKAM_DATABASE_INSTANCE, "localhost:5432/ockam");
        env::set_var(OCKAM_DATABASE_USER, "pgadmin");
        env::set_var(OCKAM_DATABASE_PASSWORD, "xR::7Zp(h|<g<Q*t:5T");
        assert_eq!(
            get_database_connection_url().unwrap(),
            Some(
                "postgres://pgadmin:xR%3A%3A7Zp%28h%7C%3Cg%3CQ%2At%3A5T@localhost:5432/ockam"
                    .into()
            ),
            "the password is url encoded"
        );

        // Now from the username_and_password variable
        env::remove_var(OCKAM_DATABASE_USER);
        env::remove_var(OCKAM_DATABASE_PASSWORD);
        env::set_var(
            OCKAM_DATABASE_USERNAME_AND_PASSWORD,
            r#"{"username":"pgadmin", "password":"xR::7Zp(h|<g<Q*t:5T"}"#,
        );
        assert_eq!(
            get_database_connection_url().unwrap(),
            Some(
                "postgres://pgadmin:xR%3A%3A7Zp%28h%7C%3Cg%3CQ%2At%3A5T@localhost:5432/ockam"
                    .into()
            ),
            "the password is url encoded"
        );
        Ok(())
    }

    #[test]
    fn test_valid_connection_strings() -> Result<()> {
        assert!(parse_connection_string("postgres://user:pass@localhost:5432/dbname").is_ok());
        Ok(())
    }

    #[test]
    fn test_invalid_connection_strings() {
        assert!(
            parse_connection_string("postgres://localhost:5432/dbname").is_err(),
            "incorrect protocol"
        );
        assert!(
            parse_connection_string("mysql://localhost:5432/dbname").is_err(),
            "incorrect protocol"
        );
        assert!(
            parse_connection_string("postgres://user@localhost:5432/dbname").is_err(),
            "missing password"
        );
        assert!(
            parse_connection_string("postgres://user:pass@host@localhost:5432/dbname").is_err(),
            "multiple @ symbols"
        );
        assert!(
            parse_connection_string("postgres://user:pass@localhost/dbname").is_err(),
            "missing port"
        );
        assert!(
            parse_connection_string("postgres://user:pass@localhost:5432").is_err(),
            "missing database name"
        );
        assert!(parse_connection_string("").is_err(), "empty string");
    }

    #[test]
    fn test_log_level_statements() {
        let configuration = DatabaseConfiguration::sqlite_in_memory().unwrap();
        assert_eq!(configuration.statements_log_level, LevelFilter::Trace)
    }
}
