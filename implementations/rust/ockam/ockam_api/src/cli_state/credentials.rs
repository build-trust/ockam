use super::Result;
use crate::cli_state::CliStateError;
use ockam_identity::{Credential, Identity, IdentityHistoryComparison};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialConfig {
    pub issuer: Identity,
    pub encoded_credential: String,
}

impl PartialEq for CredentialConfig {
    fn eq(&self, other: &Self) -> bool {
        self.encoded_credential == other.encoded_credential
            && self.issuer.compare(&other.issuer) == IdentityHistoryComparison::Equal
    }
}

impl Eq for CredentialConfig {}

impl CredentialConfig {
    pub fn new(issuer: Identity, encoded_credential: String) -> Self {
        Self {
            issuer,
            encoded_credential,
        }
    }

    pub fn credential(&self) -> Result<Credential> {
        let bytes = match hex::decode(&self.encoded_credential) {
            Ok(b) => b,
            Err(e) => {
                return Err(CliStateError::Invalid(format!(
                    "Unable to hex decode credential. {e}"
                )));
            }
        };
        minicbor::decode::<Credential>(&bytes)
            .map_err(|e| CliStateError::Invalid(format!("Unable to decode credential. {e}")))
    }
}

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;

    #[async_trait]
    impl StateDirTrait for CredentialsState {
        type Item = CredentialState;
        const DEFAULT_FILENAME: &'static str = "credential";
        const DIR_NAME: &'static str = "credentials";
        const HAS_DATA_DIR: bool = false;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
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
