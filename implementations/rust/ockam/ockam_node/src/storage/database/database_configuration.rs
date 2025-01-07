use ockam_core::compat::rand::random_string;
use ockam_core::env::get_env;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

/// Use an in-memory SQLite database
pub const OCKAM_SQLITE_IN_MEMORY: &str = "OCKAM_SQLITE_IN_MEMORY";
/// Database connection URL
pub const OCKAM_DATABASE_CONNECTION_URL: &str = "OCKAM_DATABASE_CONNECTION_URL";

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
        if let Some(connection_string) = get_env::<String>(OCKAM_DATABASE_CONNECTION_URL)? {
            check_connection_string_format(&connection_string)?;
            Ok(Some(DatabaseConfiguration::Postgres {
                connection_string: connection_string.to_owned(),
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

    /// Return the type of database that has been configured
    pub fn connection_string(&self) -> String {
        match self {
            DatabaseConfiguration::SqliteInMemory { .. } => {
                Self::create_sqlite_in_memory_connection_string()
            }
            DatabaseConfiguration::SqlitePersistent { path, .. } => {
                Self::create_sqlite_on_disk_connection_string(path)
            }
            DatabaseConfiguration::Postgres { connection_string } => connection_string.clone(),
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
