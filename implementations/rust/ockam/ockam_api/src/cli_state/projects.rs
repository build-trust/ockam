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

pub type ProjectConfig = Project;

mod traits {
    use super::*;
    use crate::cli_state::traits::*;
    use crate::cli_state::{file_stem, CliStateError};
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateTrait for ProjectsState {
        type ItemDir = ProjectState;
        type ItemConfig = ProjectConfig;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn default_filename() -> &'static str {
            "project"
        }

        fn build_dir(root_path: &Path) -> PathBuf {
            root_path.join("projects")
        }

        fn has_data_dir() -> bool {
            false
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn delete(&self, name: &str) -> Result<()> {
            // Retrieve project. If doesn't exist do nothing.
            let project = match self.get(name) {
                Ok(project) => project,
                Err(CliStateError::NotFound) => return Ok(()),
                Err(e) => return Err(e),
            };

            // If it's the default, remove link
            if let Ok(default) = self.default() {
                if default.path == project.path {
                    let _ = std::fs::remove_file(self.default_path()?);
                }
            }

            // Remove project data
            project.delete()?;

            Ok(())
        }
    }

    #[async_trait]
    impl StateItemDirTrait for ProjectState {
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

        fn delete(&self) -> Result<()> {
            std::fs::remove_file(&self.path)?;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn data_path(&self) -> Option<&PathBuf> {
            unreachable!()
        }

        fn config(&self) -> &Self::Config {
            unreachable!()
        }
    }
}
