use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use ockam::identity::Vault;
use ockam_vault_aws::AwsSigningVault;

use crate::cli_state::traits::StateItemTrait;
use crate::cli_state::{CliStateError, StateDirTrait, DATA_DIR_NAME};

use super::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultsState {
    dir: PathBuf,
}

impl VaultsState {
    pub async fn create_async(&self, name: &str, config: VaultConfig) -> Result<VaultState> {
        if self.exists(name) {
            return Err(CliStateError::AlreadyExists {
                resource: Self::default_filename().to_string(),
                name: name.to_string(),
            });
        }
        let state = VaultState::new(self.path(name), config)?;
        state.get().await?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct VaultState {
    name: String,
    path: PathBuf,
    /// The path to the vault's storage config file, contained in the data directory
    data_path: PathBuf,
    config: VaultConfig,
}

impl VaultState {
    pub async fn get(&self) -> Result<Vault> {
        if self.config.aws_kms {
            let mut vault = Vault::create();
            let aws_vault = Arc::new(AwsSigningVault::create().await?);
            vault.identity_vault = aws_vault.clone();
            vault.credential_vault = aws_vault;

            Ok(vault)
        } else {
            let vault =
                Vault::create_with_persistent_storage_path(self.vault_file_path().as_path())
                    .await?;
            Ok(vault)
        }
    }

    fn build_data_path(name: &str, path: &Path) -> PathBuf {
        path.parent()
            .expect("Should have parent")
            .join(DATA_DIR_NAME)
            .join(format!("{name}-storage.json"))
    }

    pub fn vault_file_path(&self) -> &PathBuf {
        &self.data_path
    }

    pub async fn vault(&self) -> Result<Vault> {
        let path = self.vault_file_path().clone();
        let vault = Vault::create_with_persistent_storage_path(path.as_path()).await?;
        Ok(vault)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Display for VaultState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(
            f,
            "Type: {}",
            match self.config.is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
        )?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct VaultConfig {
    #[serde(default)]
    aws_kms: bool,
}

impl VaultConfig {
    pub fn new(aws_kms: bool) -> Result<Self> {
        Ok(Self { aws_kms })
    }

    pub fn is_aws(&self) -> bool {
        self.aws_kms
    }
}

mod traits {
    use ockam_core::async_trait;

    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;

    use super::*;

    #[async_trait]
    impl StateDirTrait for VaultsState {
        type Item = VaultState;
        const DEFAULT_FILENAME: &'static str = "vault";
        const DIR_NAME: &'static str = "vaults";
        const HAS_DATA_DIR: bool = true;

        fn new(root_path: &Path) -> Self {
            Self {
                dir: Self::build_dir(root_path),
            }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn create(
            &self,
            _name: impl AsRef<str>,
            _config: <<Self as StateDirTrait>::Item as StateItemTrait>::Config,
        ) -> Result<Self::Item> {
            unreachable!()
        }

        fn delete(&self, name: impl AsRef<str>) -> Result<()> {
            // If doesn't exist do nothing.
            if !self.exists(&name) {
                return Ok(());
            }
            let vault = self.get(&name)?;
            // If it's the default, remove link
            if let Ok(default) = self.default() {
                if default.path == vault.path {
                    let _ = std::fs::remove_file(self.default_path()?);
                }
            }
            // Remove vault files
            vault.delete()?;
            Ok(())
        }
    }

    #[async_trait]
    impl StateItemTrait for VaultState {
        type Config = VaultConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, contents)?;
            let name = file_stem(&path)?;
            let data_path = VaultState::build_data_path(&name, &path);
            Ok(Self {
                name,
                path,
                data_path,
                config,
            })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let name = file_stem(&path)?;
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            let data_path = VaultState::build_data_path(&name, &path);
            Ok(Self {
                name,
                path,
                data_path,
                config,
            })
        }

        fn delete(&self) -> Result<()> {
            std::fs::remove_file(&self.path)?;
            std::fs::remove_file(&self.data_path)?;
            std::fs::remove_file(self.data_path.with_extension("json.lock"))?;
            Ok(())
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
