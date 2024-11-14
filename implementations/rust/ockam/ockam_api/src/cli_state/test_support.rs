use crate::cli_state::Result;
use crate::cli_state::{random_name, CliState, CliStateError};
use ockam_node::database::SqlxDatabase;
use std::path::PathBuf;

/// Test support
impl CliState {
    /// Return a test CliState with a random root directory
    /// Use this CliState for a simple integration test since every call to that function deletes
    /// all previous state if the database being used is Postgres.
    pub async fn test() -> Result<Self> {
        let test_dir = Self::test_dir()?;

        // clean the existing state if any
        let db = SqlxDatabase::create(&CliState::make_database_configuration(&test_dir)?).await?;
        db.drop_all_postgres_tables().await?;

        Self::create(test_dir).await
    }

    /// Return a test CliState with a random root directory
    /// Use this CliState for system tests involving several nodes
    /// since calls to that function do not delete
    /// any previous state if the database being used is Postgres.
    pub async fn system() -> Result<Self> {
        let test_dir = Self::test_dir()?;
        Self::create(test_dir).await
    }

    /// Return a random root directory
    pub fn test_dir() -> Result<PathBuf> {
        Ok(home::home_dir()
            .ok_or_else(|| CliStateError::InvalidPath("$HOME".to_string()))?
            .join(".ockam")
            .join(".tests")
            .join(random_name()))
    }
}
