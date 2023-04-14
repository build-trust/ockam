use super::Result;
use crate::cli_state::traits::StateItemDirTrait;
use ockam_identity::IdentitiesVault;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultsState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultState {
    name: String,
    path: PathBuf,
    /// The path to the vault's storage config file, contained in the data directory
    data_path: PathBuf,
    config: VaultConfig,
}

impl VaultState {
    pub async fn get(&self) -> Result<Vault> {
        let vault_storage = FileStorage::create(self.vault_file_path().clone()).await?;
        let mut vault = Vault::new(Some(Arc::new(vault_storage)));
        if self.config.aws_kms {
            vault.enable_aws_kms().await?
        }
        Ok(vault)
    }

    fn build_data_path(name: &str, path: &Path) -> PathBuf {
        path.parent()
            .expect("Should have parent")
            .join("data")
            .join(format!("{name}-storage.json"))
    }

    pub fn vault_file_path(&self) -> &PathBuf {
        self.data_path().expect("Should have data path")
    }

    pub async fn identities_vault(&self) -> Result<Arc<dyn IdentitiesVault>> {
        let path = self.vault_file_path().clone();
        Ok(Arc::new(Vault::new(Some(Arc::new(
            FileStorage::create(path).await?,
        )))))
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
    use super::*;
    use crate::cli_state::traits::*;
    use crate::cli_state::{file_stem, CliStateError};
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateTrait for VaultsState {
        type ItemDir = VaultState;
        type ItemConfig = VaultConfig;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn default_filename() -> &'static str {
            "vault"
        }

        fn build_dir(root_path: &Path) -> PathBuf {
            root_path.join("vaults")
        }

        fn has_data_dir() -> bool {
            true
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        async fn create(&self, name: &str, config: Self::ItemConfig) -> Result<Self::ItemDir> {
            let path = {
                let path = self.path(name);
                if path.exists() {
                    return Err(CliStateError::AlreadyExists);
                }
                path
            };
            let state = Self::ItemDir::new(path, config)?;
            state.get().await?;
            if !self.default_path()?.exists() {
                self.set_default(name)?;
            }
            Ok(state)
        }

        async fn delete(&self, name: &str) -> Result<()> {
            // Retrieve vault. If doesn't exist do nothing.
            let vault = match self.get(name) {
                Ok(v) => v,
                Err(CliStateError::NotFound) => return Ok(()),
                Err(e) => return Err(e),
            };

            // If it's the default, remove link
            if let Ok(default) = self.default() {
                if default.path == vault.path {
                    let _ = std::fs::remove_file(self.default_path()?);
                }
            }

            // Remove vault files
            vault.delete().await?;

            Ok(())
        }
    }

    #[async_trait]
    impl StateItemDirTrait for VaultState {
        type Config = VaultConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
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

        async fn delete(&self) -> Result<()> {
            std::fs::remove_file(&self.path)?;
            std::fs::remove_file(&self.data_path)?;
            std::fs::remove_file(self.data_path.with_extension("json.lock"))?;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn data_path(&self) -> Option<&PathBuf> {
            Some(&self.data_path)
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }

    #[async_trait]
    impl StateItemConfigTrait for VaultConfig {
        async fn delete(&self) -> Result<()> {
            unreachable!()
        }
    }
}
