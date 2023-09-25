use super::Result;
use crate::cloud::space::Space;
use crate::config::lookup::SpaceLookup;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SpacesState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SpaceState {
    name: String,
    path: PathBuf,
    config: SpaceConfig,
}

impl SpaceState {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpaceConfig {
    pub name: String,
    pub id: String,
}

impl SpaceConfig {
    pub fn from_lookup(name: &str, lookup: SpaceLookup) -> Self {
        Self {
            name: name.to_string(),
            id: lookup.id,
        }
    }
}

impl From<&Space> for SpaceConfig {
    fn from(s: &Space) -> Self {
        Self {
            name: s.name.to_string(),
            id: s.id.to_string(),
        }
    }
}

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateDirTrait for SpacesState {
        type Item = SpaceState;
        const DEFAULT_FILENAME: &'static str = "space";
        const DIR_NAME: &'static str = "spaces";
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
    impl StateItemTrait for SpaceState {
        type Config = SpaceConfig;

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
