use ockam_core::compat::rand::random_string;
use ockam_core::env::get_env;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

/// Use an in-memory SQLite database
pub const OCKAM_SQLITE_IN_MEMORY: &str = "OCKAM_SQLITE_IN_MEMORY";
/// Database host environment variable
pub const OCKAM_POSTGRES_HOST: &str = "OCKAM_POSTGRES_HOST";
/// Database port environment variable
pub const OCKAM_POSTGRES_PORT: &str = "OCKAM_POSTGRES_PORT";
/// Database name environment variable
pub const OCKAM_POSTGRES_DATABASE_NAME: &str = "OCKAM_POSTGRES_DATABASE_NAME";
/// Database user environment variable
pub const OCKAM_POSTGRES_USER: &str = "OCKAM_POSTGRES_USER";
/// Database password environment variable
pub const OCKAM_POSTGRES_PASSWORD: &str = "OCKAM_POSTGRES_PASSWORD";

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
        /// Database host name
        host: String,
        /// Database host port
        port: u16,
        /// Database name
        database_name: String,
        /// Database user
        user: Option<DatabaseUser>,
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
    /// Create a postgres database configuration from environment variables.
    ///
    /// At minima, the database host and port must be provided.
    pub fn postgres() -> Result<Option<DatabaseConfiguration>> {
        let host: Option<String> = get_env(OCKAM_POSTGRES_HOST)?;
        let port: Option<u16> = get_env(OCKAM_POSTGRES_PORT)?;
        let database_name: String =
            get_env(OCKAM_POSTGRES_DATABASE_NAME)?.unwrap_or("postgres".to_string());
        let user: Option<String> = get_env(OCKAM_POSTGRES_USER)?;
        let password: Option<String> = get_env(OCKAM_POSTGRES_PASSWORD)?;
        match (host, port) {
            (Some(host), Some(port)) => match (user, password) {
                (Some(user), Some(password)) => Ok(Some(DatabaseConfiguration::Postgres {
                    host,
                    port,
                    database_name,
                    user: Some(DatabaseUser::new(user, password)),
                })),
                _ => Ok(Some(DatabaseConfiguration::Postgres {
                    host,
                    port,
                    database_name,
                    user: None,
                })),
            },
            _ => Ok(None),
        }
    }

    /// Create a local sqlite configuration
    pub fn sqlite(path: &Path) -> DatabaseConfiguration {
        DatabaseConfiguration::SqlitePersistent {
            path: path.to_path_buf(),
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
            DatabaseConfiguration::Postgres {
                host,
                port,
                database_name,
                user,
            } => Self::create_postgres_connection_string(
                host.clone(),
                *port,
                database_name.clone(),
                user.clone(),
            ),
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

    fn create_postgres_connection_string(
        host: String,
        port: u16,
        database_name: String,
        user: Option<DatabaseUser>,
    ) -> String {
        let user_password = match user {
            Some(user) => format!("{}:{}@", user.user_name(), user.password()),
            None => "".to_string(),
        };
        format!("postgres://{user_password}{host}:{port}/{database_name}")
    }
}
