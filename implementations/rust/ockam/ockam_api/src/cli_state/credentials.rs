use super::Result;
use crate::cli_state::CliStateError;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::Identifier;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CredentialsState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CredentialState {
    name: String,
    path: PathBuf,
    config: CredentialConfig,
}

impl CredentialState {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialConfig {
    pub issuer_identifier: Identifier,
    // FIXME: Appear as array of number in JSON
    pub encoded_issuer_change_history: Vec<u8>,
    // FIXME: Appear as array of number in JSON
    pub encoded_credential: Vec<u8>,
}

impl CredentialConfig {
    pub fn new(
        issuer_identifier: Identifier,
        encoded_issuer_change_history: Vec<u8>,
        encoded_credential: Vec<u8>,
    ) -> Result<Self> {
        Ok(Self {
            issuer_identifier,
            encoded_issuer_change_history,
            encoded_credential,
        })
    }

    pub fn credential(&self) -> Result<CredentialAndPurposeKey> {
        minicbor::decode(&self.encoded_credential).map_err(|e| {
            error!(%e, "Unable to decode credential");
            CliStateError::InvalidOperation("Unable to decode credential".to_string())
        })
    }
}

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateDirTrait for CredentialsState {
        type Item = CredentialState;
        const DEFAULT_FILENAME: &'static str = "credential";
        const DIR_NAME: &'static str = "credentials";
        const HAS_DATA_DIR: bool = false;

        fn new(root_path: &Path) -> Self {
            Self {
                dir: Self::build_dir(root_path),
            }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }
    }

    #[async_trait]
    impl StateItemTrait for CredentialState {
        type Config = CredentialConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            let name = file_stem(&path)?;
            Ok(Self { name, path, config })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let name = file_stem(&path)?;
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            Ok(Self { name, path, config })
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
