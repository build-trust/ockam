use rand::random;
use std::path::{Path, PathBuf};
use tokio::sync::broadcast::{channel, Receiver, Sender};

use ockam::SqlxDatabase;
use ockam_core::env::get_env_with_default;
use ockam_node::database::DatabaseConfiguration;
use ockam_node::Executor;

use crate::cli_state::error::Result;
use crate::cli_state::CliStateError;
use crate::logs::ExportingEnabled;
use crate::terminal::notification::Notification;

/// Maximum number of notifications present in the channel
const NOTIFICATIONS_CHANNEL_CAPACITY: usize = 16;

/// The CliState struct manages all the data persisted locally.
///
/// The data is saved to several files:
///
/// - The "nodes" database file. That file contains most of the configuration for the nodes running locally: project, node,
///   inlets, outlets, etc... That file is deleted when the `ockam reset` command is executed
///
/// - The "application" database file. That file stores the tracing data which needs to persist across all commands
///   including reset
///
/// - One file per additional vault created with the `ockam vault create` command
///
/// The database files are accessed with the SqlxDatabase struct, and use different migration files to define their
/// schema.
///
/// On top of each SqlxDatabase, there are different repositories. A Repository encapsulates SQL queries for
/// creating / updating / deleting entities. Some examples of entities that are persisted: Project, Space, Vault, Identity, etc...
///
/// The repositories themselves are not accessible from the `CliState` directly since it is often
/// necessary to use more than one repository to implement a given behaviour. For example deleting
/// an identity requires to query the nodes that are using that identity and only delete it if no
/// node is using that identity
///
#[derive(Debug, Clone)]
pub struct CliState {
    dir: PathBuf,
    database: SqlxDatabase,
    application_database: SqlxDatabase,
    exporting_enabled: ExportingEnabled,
    /// Broadcast channel to be notified of major events during a process supported by the
    /// CliState API
    notifications: Sender<Notification>,
}

impl CliState {
    /// Create a new CliState in a given directory
    pub fn new(dir: &Path) -> Result<Self> {
        Executor::execute_future(Self::create(dir.into()))?
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.clone()
    }

    pub fn database(&self) -> SqlxDatabase {
        self.database.clone()
    }

    pub fn database_ref(&self) -> &SqlxDatabase {
        &self.database
    }

    pub fn database_configuration(&self) -> Result<DatabaseConfiguration> {
        Self::make_database_configuration(&self.dir)
    }

    pub fn is_database_path(&self, path: &Path) -> bool {
        let database_configuration = self.database_configuration().ok();
        match database_configuration {
            Some(c) => c.path() == Some(path.to_path_buf()),
            None => false,
        }
    }

    pub fn application_database(&self) -> SqlxDatabase {
        self.application_database.clone()
    }

    pub fn application_database_configuration(&self) -> Result<DatabaseConfiguration> {
        Self::make_application_database_configuration(&self.dir)
    }

    pub fn subscribe_to_notifications(&self) -> Receiver<Notification> {
        self.notifications.subscribe()
    }

    pub fn notify_message(&self, message: impl Into<String>) {
        self.notify(Notification::message(message));
    }

    pub fn notify_progress(&self, message: impl Into<String>) {
        self.notify(Notification::progress(message));
    }

    pub fn notify_progress_finish(&self, message: impl Into<String>) {
        self.notify(Notification::progress_finish(Some(message.into())));
    }

    pub fn notify_progress_finish_and_clear(&self) {
        self.notify(Notification::progress_finish(None));
    }

    fn notify(&self, notification: Notification) {
        debug!("{:?}", notification.contents());
        let _ = self.notifications.send(notification);
    }
}

/// These functions allow to create and reset the local state
impl CliState {
    /// Return a new CliState using a default directory to store its data
    pub fn with_default_dir() -> Result<Self> {
        Self::new(Self::default_dir()?.as_path())
    }

    /// Stop nodes and remove all the directories storing state
    pub async fn reset(&self) -> Result<()> {
        self.delete_all_named_identities().await?;
        self.delete_all_nodes().await?;
        self.delete_all_named_vaults().await?;
        self.delete().await
    }

    /// Removes all the directories storing state without loading the current state
    /// The database data is only removed if the database is a SQLite one
    pub fn hard_reset() -> Result<()> {
        let dir = Self::default_dir()?;
        Self::delete_at(&dir)
    }

    /// Delete the local database and log files
    pub async fn delete(&self) -> Result<()> {
        self.database.drop_postgres_node_tables().await?;
        self.delete_local_data()
    }

    /// Delete the local data on disk: sqlite database file and log files
    pub fn delete_local_data(&self) -> Result<()> {
        Self::delete_at(&self.dir)
    }

    /// Reset all directories and return a new CliState
    pub async fn recreate(&self) -> Result<CliState> {
        self.reset().await?;
        Self::create(self.dir.clone()).await
    }

    /// Backup and reset is used to save aside
    /// some corrupted local state for later inspection and then reset the state.
    /// The database is backed-up only if it is a SQLite database.
    pub fn backup_and_reset() -> Result<()> {
        let dir = Self::default_dir()?;

        // Reset backup directory
        let backup_dir = Self::backup_default_dir()?;
        if backup_dir.exists() {
            let _ = std::fs::remove_dir_all(&backup_dir);
        }
        std::fs::create_dir_all(&backup_dir)?;

        // Move state to backup directory
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let from = entry.path();
            let to = backup_dir.join(entry.file_name());
            std::fs::rename(from, to)?;
        }

        // Reset state
        Self::delete_at(&dir)?;
        let state = Self::new(&dir)?;

        let dir = &state.dir;
        let backup_dir = CliState::backup_default_dir().unwrap();
        eprintln!("The {dir:?} directory has been reset and has been backed up to {backup_dir:?}");
        Ok(())
    }

    /// Returns the default backup directory for the CLI state.
    pub fn backup_default_dir() -> Result<PathBuf> {
        let dir = Self::default_dir()?;
        let dir_name =
            dir.file_name()
                .and_then(|n| n.to_str())
                .ok_or(CliStateError::InvalidOperation(
                    "The $OCKAM_HOME directory does not have a valid name".to_string(),
                ))?;
        let parent = dir.parent().ok_or(CliStateError::InvalidOperation(
            "The $OCKAM_HOME directory does not a valid parent directory".to_string(),
        ))?;
        Ok(parent.join(format!("{dir_name}.bak")))
    }
}

/// Low-level functions for creating / deleting CliState files
impl CliState {
    /// Create a new CliState where the data is stored at a given path
    pub async fn create(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        let database = SqlxDatabase::create(&Self::make_database_configuration(&dir)?).await?;
        let configuration = Self::make_application_database_configuration(&dir)?;
        let application_database =
            SqlxDatabase::create_application_database(&configuration).await?;
        debug!("Opened the main database with options {:?}", database);
        debug!(
            "Opened the application database with options {:?}",
            application_database
        );
        let (notifications, _) = channel::<Notification>(NOTIFICATIONS_CHANNEL_CAPACITY);
        let state = Self {
            dir,
            database,
            application_database,
            // We initialize the CliState with no tracing.
            // Once the logging/tracing options have been determined, then
            // the function set_tracing_enabled can be used to enable tracing, which
            // is eventually used to trace user journeys.
            exporting_enabled: ExportingEnabled::Off,
            notifications,
        };
        Ok(state)
    }

    pub fn is_tracing_enabled(&self) -> bool {
        self.exporting_enabled == ExportingEnabled::On
    }

    pub fn set_tracing_enabled(self, enabled: bool) -> CliState {
        CliState {
            exporting_enabled: if enabled {
                ExportingEnabled::On
            } else {
                ExportingEnabled::Off
            },
            ..self
        }
    }

    /// If the postgres database is configured, return the postgres configuration
    pub(super) fn make_database_configuration(root_path: &Path) -> Result<DatabaseConfiguration> {
        match DatabaseConfiguration::postgres()? {
            Some(configuration) => Ok(configuration),
            None => Ok(DatabaseConfiguration::sqlite(
                root_path.join("database.sqlite3").as_path(),
            )),
        }
    }

    /// If the postgres database is configured, return the postgres configuration
    pub(super) fn make_application_database_configuration(
        root_path: &Path,
    ) -> Result<DatabaseConfiguration> {
        match DatabaseConfiguration::postgres()? {
            Some(configuration) => Ok(configuration),
            None => Ok(DatabaseConfiguration::sqlite(
                root_path.join("application_database.sqlite3").as_path(),
            )),
        }
    }

    pub(super) fn make_node_dir_path(root_path: &Path, node_name: &str) -> PathBuf {
        Self::make_nodes_dir_path(root_path).join(node_name)
    }

    pub(super) fn make_command_log_path(root_path: &Path, command_name: &str) -> PathBuf {
        Self::make_commands_log_dir_path(root_path).join(command_name)
    }

    pub(super) fn make_nodes_dir_path(root_path: &Path) -> PathBuf {
        root_path.join("nodes")
    }

    pub(super) fn make_commands_log_dir_path(root_path: &Path) -> PathBuf {
        root_path.join("commands")
    }

    /// Delete the state files
    fn delete_at(root_path: &Path) -> Result<()> {
        // Delete nodes logs
        let _ = std::fs::remove_dir_all(Self::make_nodes_dir_path(root_path));
        // Delete command logs
        let _ = std::fs::remove_dir_all(Self::make_commands_log_dir_path(root_path));
        // Delete the nodes database, keep the application database
        if let Some(path) = Self::make_database_configuration(root_path)?.path() {
            std::fs::remove_file(path)?;
        };
        Ok(())
    }

    /// Returns the default directory for the CLI state.
    /// That directory is determined by `OCKAM_HOME` environment variable and is
    /// $OCKAM_HOME/.ockam.
    ///
    /// If $OCKAM_HOME is not defined then $HOME is used instead
    pub(super) fn default_dir() -> Result<PathBuf> {
        Ok(get_env_with_default::<PathBuf>(
            "OCKAM_HOME",
            home::home_dir()
                .ok_or(CliStateError::InvalidPath("$HOME".to_string()))?
                .join(".ockam"),
        )?)
    }
}

/// Return a random, but memorable, name which can be used to name identities, nodes, vaults, etc...
pub fn random_name() -> String {
    petname::petname(2, "-").unwrap_or(hex::encode(random::<[u8; 4]>()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use ockam_node::database::DatabaseType;
    use sqlx::any::AnyRow;
    use sqlx::Row;
    use std::fs;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_reset() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let cli_state_directory = db_file.path().parent().unwrap().join(random_name());
        let db = SqlxDatabase::create(&CliState::make_database_configuration(
            &cli_state_directory,
        )?)
        .await?;
        db.drop_all_postgres_tables().await?;
        let cli = CliState::create(cli_state_directory.clone()).await?;

        // create 2 vaults
        // the second vault is using a separate file
        let _vault1 = cli.get_or_create_named_vault("vault1").await?;
        let _vault2 = cli.get_or_create_named_vault("vault2").await?;

        // create 2 identities
        let identity1 = cli
            .create_identity_with_name_and_vault("identity1", "vault1")
            .await?;
        let identity2 = cli
            .create_identity_with_name_and_vault("identity2", "vault2")
            .await?;

        // create 2 nodes
        let _node1 = cli
            .create_node_with_identifier("node1", &identity1.identifier())
            .await?;
        let _node2 = cli
            .create_node_with_identifier("node2", &identity2.identifier())
            .await?;

        let file_names = list_file_names(&cli_state_directory);
        let expected = match cli.database_configuration()?.database_type() {
            DatabaseType::Sqlite => vec![
                "vault-vault2".to_string(),
                "application_database.sqlite3".to_string(),
                "database.sqlite3".to_string(),
            ],
            DatabaseType::Postgres => vec!["vault-vault2".to_string()],
        };

        assert_eq!(
            file_names.iter().sorted().as_slice(),
            expected.iter().sorted().as_slice()
        );

        // reset the local state
        cli.reset().await?;
        let result = fs::read_dir(&cli_state_directory);
        assert!(result.is_ok(), "the cli state directory is not deleted");

        match cli.database_configuration()?.database_type() {
            DatabaseType::Sqlite => {
                // When the database is SQLite, only the application database must remain
                let file_names = list_file_names(&cli_state_directory);
                let expected = vec!["application_database.sqlite3".to_string()];
                assert_eq!(file_names, expected);
            }
            DatabaseType::Postgres => {
                // When the database is Postgres, only the journey tables must remain
                let tables: Vec<AnyRow> = sqlx::query(
                    "SELECT tablename::text FROM pg_tables WHERE schemaname = 'public'",
                )
                .fetch_all(&*db.pool)
                .await
                .unwrap();
                let actual: Vec<String> = tables.iter().map(|r| r.get(0)).sorted().collect();
                assert_eq!(actual, vec!["host_journey", "project_journey"]);
            }
        };
        Ok(())
    }

    /// HELPERS
    fn list_file_names(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|f| f.unwrap().file_name().to_string_lossy().to_string())
            .collect()
    }

    #[tokio::test]
    async fn random_name_generator() {
        // Same process
        let mut names = Vec::new();
        for _ in 0..100 {
            names.push(random_name());
        }
        assert_eq!(names.len(), names.iter().unique().count());

        // In an async context
        let mut names = Vec::new();
        for _ in 0..100 {
            names.push(tokio::task::spawn_blocking(random_name).await.unwrap());
        }
        assert_eq!(names.len(), names.iter().unique().count());
    }
}
