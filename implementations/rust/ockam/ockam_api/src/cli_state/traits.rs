use crate::cli_state::{file_stem, CliState, CliStateError};
use ockam_core::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::Result;

/// Represents the directory of a type of state. This directory contains a list of items, uniquely
/// identified by a name, and represented by the same `Item` type.
///
/// One item can be set as the "default" item, which is used in some CLI commands when no
/// argument is provided for that type of `Item`.
#[async_trait]
pub trait StateDirTrait: Sized + Send + Sync {
    type Item: StateItemTrait;
    const DEFAULT_FILENAME: &'static str;
    const DIR_NAME: &'static str;
    const HAS_DATA_DIR: bool;

    fn new(dir: PathBuf) -> Self;

    fn default_filename() -> &'static str {
        Self::DEFAULT_FILENAME
    }
    fn build_dir(root_path: &Path) -> PathBuf {
        root_path.join(Self::DIR_NAME)
    }
    fn has_data_dir() -> bool {
        Self::HAS_DATA_DIR
    }

    /// Load the root configuration
    /// and migrate each entry if necessary
    async fn init(root_path: &Path) -> Result<Self> {
        let root = Self::load(root_path)?;
        let root_dir = root.dir();
        for entry in std::fs::read_dir(root_dir)? {
            let item_path = root.path(&file_stem(&entry?.path())?);
            root.migrate(item_path.as_path()).await?;
        }
        Ok(root)
    }

    async fn migrate(&self, _item_path: &Path) -> Result<()> {
        Ok(())
    }

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

    fn overwrite(
        &self,
        name: &str,
        config: <<Self as StateDirTrait>::Item as StateItemTrait>::Config,
    ) -> Result<Self::Item> {
        let path = self.path(name);
        let state = Self::Item::new(path, config)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }

    fn create(
        &self,
        name: &str,
        config: <<Self as StateDirTrait>::Item as StateItemTrait>::Config,
    ) -> Result<Self::Item> {
        if self.exists(name) {
            return Err(CliStateError::AlreadyExists);
        }
        let state = Self::Item::new(self.path(name), config)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }

    fn get(&self, name: &str) -> Result<Self::Item> {
        if !self.exists(name) {
            return Err(CliStateError::NotFound);
        }
        Self::Item::load(self.path(name))
    }

    fn list(&self) -> Result<Vec<Self::Item>> {
        let mut items = Vec::default();
        for entry in std::fs::read_dir(self.dir())? {
            let name = file_stem(&entry?.path())?;
            if let Ok(item) = self.get(&name) {
                items.push(item);
            }
        }
        Ok(items)
    }

    // TODO: move to StateItemTrait
    fn delete(&self, name: &str) -> Result<()> {
        // Retrieve state. If doesn't exist do nothing.
        let s = match self.get(name) {
            Ok(project) => project,
            Err(CliStateError::NotFound) => return Ok(()),
            Err(e) => return Err(e),
        };
        // If it's the default, remove link
        if let Ok(default) = self.default() {
            if default.path() == s.path() {
                let _ = std::fs::remove_file(self.default_path()?);
            }
        }
        // Remove state data
        s.delete()
    }

    fn default_path(&self) -> Result<PathBuf> {
        let root_path = self.dir().parent().expect("Should have parent");
        Ok(CliState::defaults_dir(root_path)?.join(Self::default_filename()))
    }

    fn default(&self) -> Result<Self::Item> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        Self::Item::load(path)
    }

    fn set_default(&self, name: &str) -> Result<()> {
        if !self.exists(name) {
            return Err(CliStateError::NotFound);
        }
        let original = self.path(name);
        let link = self.default_path()?;
        // Remove link if it exists
        let _ = std::fs::remove_file(&link);
        // Create link to the identity
        std::os::unix::fs::symlink(original, link)?;
        Ok(())
    }

    fn is_default(&self, name: &str) -> Result<bool> {
        if !self.exists(name) {
            return Ok(false);
        }
        let default_name = {
            let path = std::fs::canonicalize(self.default_path()?)?;
            file_stem(&path)?
        };
        Ok(default_name.eq(name))
    }

    fn is_empty(&self) -> Result<bool> {
        for entry in std::fs::read_dir(self.dir())? {
            let name = file_stem(&entry?.path())?;
            if self.get(&name).is_ok() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn exists(&self, name: &str) -> bool {
        self.path(name).exists()
    }
}

/// This trait defines the methods to retrieve an item from a state directory.
/// The details of the item are defined in the `Config` type.
#[async_trait]
pub trait StateItemTrait: Sized + Send {
    type Config: Serialize + for<'a> Deserialize<'a> + Send;

    /// Create a new item with the given config.
    fn new(path: PathBuf, config: Self::Config) -> Result<Self>;

    /// Load an item from the given path.
    fn load(path: PathBuf) -> Result<Self>;

    /// Persist the item to disk after updating the config.
    fn persist(&self) -> Result<()> {
        let contents = serde_json::to_string(self.config())?;
        std::fs::write(self.path(), contents)?;
        Ok(())
    }
    fn delete(&self) -> Result<()> {
        std::fs::remove_file(self.path())?;
        Ok(())
    }
    fn path(&self) -> &PathBuf;
    fn config(&self) -> &Self::Config;
}
