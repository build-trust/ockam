use crate::cli_state::Result;
use crate::cli_state::{random_name, CliState, CliStateError};
use std::path::PathBuf;

/// Test support
impl CliState {
    /// Return a test CliState with a random root directory
    pub async fn test() -> Result<Self> {
        let mut state = Self::create(Self::test_dir()?).await?;
        state.set_node_name(random_name());
        Ok(state)
    }

    /// Return a random root directory
    pub fn test_dir() -> Result<PathBuf> {
        Ok(home::home_dir()
            .ok_or(CliStateError::InvalidPath("$HOME".to_string()))?
            .join(".ockam")
            .join(".tests")
            .join(random_name()))
    }
}
