use ockam_core::compat::rand::random_string;
use ockam_core::env::get_env;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use percent_encoding::NON_ALPHANUMERIC;
use serde_json::Value;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

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

/// Configuration for the database.
/// We either use Sqlite or Postgres
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabaseConfiguration {
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
        /// Connection string of the form postgres://[{user}:{password}@]{host}:{port}/{database_name}
        connection_string: String,
        /// Path to a SQLite database that needs to be migrated to the Postgres database.
        legacy_sqlite_path: Option<PathBuf>,
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
            Ok(Some(DatabaseConfiguration::Postgres {
                connection_string: connection_string.to_owned(),
                legacy_sqlite_path: sqlite_path,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a local sqlite configuration
    pub fn sqlite(path: impl AsRef<Path>) -> DatabaseConfiguration {
        DatabaseConfiguration::SqlitePersistent {
            path: path.as_ref().to_path_buf(),
            single_connection: false,
        }
    }

    /// Create an in-memory sqlite configuration
    pub fn sqlite_in_memory() -> DatabaseConfiguration {
        DatabaseConfiguration::SqliteInMemory {
            single_connection: false,
        }
    }

    /// Create a single connection sqlite configuration
    pub fn single_connection(&self) -> Self {
        match self {
            DatabaseConfiguration::SqlitePersistent { path, .. } => {
                DatabaseConfiguration::SqlitePersistent {
                    path: path.clone(),
                    single_connection: true,
                }
            }
            DatabaseConfiguration::SqliteInMemory { .. } => DatabaseConfiguration::SqliteInMemory {
                single_connection: true,
            },
            _ => self.clone(),
        }
    }

    /// Return the type of database that has been configured
    pub fn database_type(&self) -> DatabaseType {
        match self {
            DatabaseConfiguration::SqliteInMemory { .. } => DatabaseType::Sqlite,
            DatabaseConfiguration::SqlitePersistent { .. } => DatabaseType::Sqlite,
            DatabaseConfiguration::Postgres { .. } => DatabaseType::Postgres,
        }
    }

    /// Return the legacy sqlite path if any
    pub fn legacy_sqlite_path(&self) -> Option<PathBuf> {
        match self {
            DatabaseConfiguration::SqliteInMemory { .. } => None,
            DatabaseConfiguration::SqlitePersistent { .. } => None,
            DatabaseConfiguration::Postgres {
                legacy_sqlite_path, ..
            } => legacy_sqlite_path.clone(),
        }
    }

    /// Return the type of database that has been configured
    pub fn connection_string(&self) -> String {
        match self {
            DatabaseConfiguration::SqliteInMemory { .. } => {
                Self::create_sqlite_in_memory_connection_string()
            }
            DatabaseConfiguration::SqlitePersistent { path, .. } => {
                Self::create_sqlite_on_disk_connection_string(path)
            }
            DatabaseConfiguration::Postgres {
                connection_string, ..
            } => connection_string.clone(),
        }
    }

    /// Create a directory for the SQLite database file if necessary
    pub fn create_directory_if_necessary(&self) -> Result<()> {
        if let DatabaseConfiguration::SqlitePersistent { path, .. } = self {
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
        match self {
            DatabaseConfiguration::SqlitePersistent { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    fn create_sqlite_in_memory_connection_string() -> String {
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
    check_connection_string_format(&connection_string)?;
    Ok(Some(connection_string))
}

/// Check the format of a database connection string as `postgres://[{user}:{password}@]{host}:{port}/{database_name}`
/// For now we only support postgres.
fn check_connection_string_format(connection_string: &str) -> Result<()> {
    if let Some(no_prefix) = connection_string.strip_prefix("postgres://") {
        let host_port_db_name = match no_prefix.split('@').collect::<Vec<_>>()[..] {
            [host_port_db_name] => host_port_db_name,
            [user_and_password, host_port_db_name] => {
                let user_and_password = user_and_password.split(':').collect::<Vec<_>>();
                if user_and_password.len() != 2 {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::Invalid,
                        "A database connection URL must specify the user and password as user:password".to_string(),
                    ));
                }
                host_port_db_name
            }
            _ => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    "A database connection URL can only have one @ separator to specify the user name and password".to_string(),
                ));
            }
        };
        match host_port_db_name.split('/').collect::<Vec<_>>()[..] {
            [host_port, _] => {
                let host_port = host_port.split(':').collect::<Vec<_>>();
                if host_port.len() != 2 {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::Invalid,
                        "A database connection URL must have a host and a port specified as host:port".to_string(),
                    ));
                }
                Ok(())
            }
            _ => Err(Error::new(
                Origin::Api,
                Kind::Invalid,
                "A database connection URL must have a host, a port and a database name as host:port/database_name".to_string(),
            )),
        }
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
        Ok(())
    }

    #[test]
    fn test_make_connection_url_from_separate_env_variables_user_and_password() -> Result<()> {
        env::set_var(OCKAM_DATABASE_INSTANCE, "localhost:5432/ockam");
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
        assert!(
            check_connection_string_format("postgres://user:pass@localhost:5432/dbname").is_ok()
        );
        assert!(check_connection_string_format("postgres://localhost:5432/dbname").is_ok());
        Ok(())
    }

    #[test]
    fn test_invalid_connection_strings() {
        assert!(
            check_connection_string_format("mysql://localhost:5432/dbname").is_err(),
            "incorrect protocol"
        );
        assert!(
            check_connection_string_format("postgres://user@localhost:5432/dbname").is_err(),
            "missing password"
        );
        assert!(
            check_connection_string_format("postgres://user:pass@host@localhost:5432/dbname")
                .is_err(),
            "multiple @ symbols"
        );
        assert!(
            check_connection_string_format("postgres://user:pass@localhost/dbname").is_err(),
            "missing port"
        );
        assert!(
            check_connection_string_format("postgres://user:pass@localhost:5432").is_err(),
            "missing database name"
        );
        assert!(check_connection_string_format("").is_err(), "empty string");
    }
}
