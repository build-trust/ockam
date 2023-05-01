use super::Result;
use crate::cloud::project::Project;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectsState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectState {
    name: String,
    path: PathBuf,
}

impl ProjectState {
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub type ProjectConfig = Project;

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;

    #[async_trait]
    impl StateDirTrait for ProjectsState {
        type Item = ProjectState;
        const DEFAULT_FILENAME: &'static str = "project";
        const DIR_NAME: &'static str = "projects";
        const HAS_DATA_DIR: bool = false;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }
    }

    #[async_trait]
    impl StateItemTrait for ProjectState {
        type Config = ProjectConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            let name = file_stem(&path)?;
            Ok(Self { name, path })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let name = file_stem(&path)?;
            Ok(Self { name, path })
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            unreachable!()
        }
    }
}
