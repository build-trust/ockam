use crate::cli_state::{file_stem, CliState, CliStateError};
use ockam_core::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::Result;

/// Represents the directory of a type of state and contains
/// all the data related to that type.
#[async_trait]
pub trait StateTrait: Sized {
    type ItemDir: StateItemDirTrait;
    type ItemConfig: StateItemConfigTrait + Serialize + for<'a> Deserialize<'a>;

    fn new(dir: PathBuf) -> Self;
    fn default_filename() -> &'static str;
    fn build_dir(root_path: &Path) -> PathBuf;
    fn has_data_dir() -> bool;

    #[allow(clippy::new_ret_no_self)]
    fn load(root_path: &Path) -> Result<Self> {
        let dir = Self::build_dir(root_path);
        if Self::has_data_dir() {
            std::fs::create_dir_all(dir.join("data"))?;
        } else {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(Self::new(dir))
    }

    fn dir(&self) -> &PathBuf;
    fn dir_as_string(&self) -> String {
        self.dir().to_string_lossy().to_string()
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir().join(format!("{name}.json"))
    }

    async fn create(&self, name: &str, config: Self::ItemConfig) -> Result<Self::ItemDir>;

    fn get(&self, name: &str) -> Result<Self::ItemDir> {
        let path = {
            let path = self.path(name);
            if !path.exists() {
                return Err(CliStateError::NotFound);
            }
            path
        };
        Self::ItemDir::load(path)
    }

    fn list(&self) -> Result<Vec<Self::ItemDir>> {
        let mut items = Vec::default();
        for entry in std::fs::read_dir(self.dir())? {
            if let Ok(item) = self.get(&file_stem(&entry?.path())?) {
                items.push(item);
            }
        }
        Ok(items)
    }

    async fn delete(&self, name: &str) -> Result<()>;

    fn default_path(&self) -> Result<PathBuf> {
        let root_path = self.dir().parent().expect("Should have parent");
        Ok(CliState::defaults_dir(root_path)?.join(Self::default_filename()))
    }

    fn default(&self) -> Result<Self::ItemDir> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        Self::ItemDir::load(path)
    }

    fn set_default(&self, name: &str) -> Result<()> {
        let original = {
            let path = self.path(name);
            if !path.exists() {
                return Err(CliStateError::NotFound);
            }
            path
        };
        let link = self.default_path()?;
        // Remove link if it exists
        let _ = std::fs::remove_file(&link);
        // Create link to the identity
        std::os::unix::fs::symlink(original, link)?;
        Ok(())
    }

    fn is_default(&self, name: &str) -> Result<bool> {
        let _exists = self.get(name)?;
        let default_name = {
            let path = std::fs::canonicalize(self.default_path()?)?;
            file_stem(&path)?
        };
        Ok(default_name.eq(name))
    }

    fn has_default(&self) -> Result<bool> {
        Ok(self.default_path()?.exists())
    }
}

/// This trait defines the methods to retrieve an item from a state directory.
/// The details of the item are defined in the `Config` type.
#[async_trait]
pub trait StateItemDirTrait: Sized {
    type Config: StateItemConfigTrait + Serialize + for<'a> Deserialize<'a>;

    fn new(path: PathBuf, config: Self::Config) -> Result<Self>;
    fn load(path: PathBuf) -> Result<Self>;
    async fn delete(&self) -> Result<()>;
    fn name(&self) -> &str;
    fn path(&self) -> &PathBuf;
    fn data_path(&self) -> Option<&PathBuf>;
    fn config(&self) -> &Self::Config;
}

#[async_trait]
pub trait StateItemConfigTrait {
    async fn delete(&self) -> Result<()>;
}
