use crate::cli_state::{file_stem, CliState, CliStateError};
use ockam_core::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::Result;

/// Represents the directory of a type of state and contains
/// all the data related to that type.
#[async_trait]
pub trait StateDirTrait: Sized {
    type Item: StateItemTrait;

    fn cli_state(&self) -> Result<CliState> {
        CliState::new(self.dir().parent().expect("no parent dir"))
    }

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

    async fn create_async(
        &self,
        _name: &str,
        _config: <<Self as StateDirTrait>::Item as StateItemTrait>::Config,
    ) -> Result<Self::Item> {
        unreachable!()
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
            if let Ok(item) = self.get(&file_stem(&entry?.path())?) {
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
        s.delete()?;
        Ok(())
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
            return Err(CliStateError::NotFound);
        }
        let default_name = {
            let path = std::fs::canonicalize(self.default_path()?)?;
            file_stem(&path)?
        };
        Ok(default_name.eq(name))
    }

    fn has_default(&self) -> Result<bool> {
        Ok(self.default_path()?.exists())
    }

    fn exists(&self, name: &str) -> bool {
        self.path(name).exists()
    }
}

/// This trait defines the methods to retrieve an item from a state directory.
/// The details of the item are defined in the `Config` type.
#[async_trait]
pub trait StateItemTrait: Sized {
    type Config: Serialize + for<'a> Deserialize<'a> + Send;

    fn cli_state(&self) -> Result<CliState> {
        CliState::new(
            self.path()
                .parent()
                .expect("no parent dir")
                .parent()
                .expect("no parent dir"),
        )
    }

    fn new(path: PathBuf, config: Self::Config) -> Result<Self>;
    fn load(path: PathBuf) -> Result<Self>;
    fn persist(&self) -> Result<()> {
        let contents = serde_json::to_string(self.config())?;
        std::fs::write(self.path(), contents)?;
        Ok(())
    }
    fn delete(&self) -> Result<()> {
        std::fs::remove_file(self.path())?;
        Ok(())
    }
    fn name(&self) -> &str;
    fn path(&self) -> &PathBuf;
    fn data_path(&self) -> Option<&PathBuf>;
    fn config(&self) -> &Self::Config;
}
