use super::Result;
use crate::cloud::project::Project;
use crate::config::lookup::ProjectLookup;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectsState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectState {
    path: PathBuf,
    config: ProjectConfig,
}

impl ProjectState {
    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }
}

pub type ProjectConfig = Project;

impl From<ProjectLookup> for Project {
    fn from(lookup: ProjectLookup) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            id: lookup.id,
            name: lookup.name,
            space_name: "".to_string(),
            access_route: lookup
                .node_route
                .map(|r| r.to_string())
                .unwrap_or("".to_string()),
            users: vec![],
            space_id: "".to_string(),
            identity: lookup.identity_id,
            authority_access_route: lookup.authority.as_ref().map(|a| a.address().to_string()),
            authority_identity: lookup.authority.as_ref().map(|a| hex::encode(a.identity())),
            okta_config: lookup.okta.map(|o| o.into()),
            confluent_config: None,
            version: None,
            running: None,
            operation_id: None,
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

        async fn migrate(&self, path: &Path) -> Result<()> {
            // Rename the project file using the project id
            let project = match ProjectState::load(path.to_path_buf()) {
                Ok(p) => p,
                Err(_) => {
                    // Skip unexpected files
                    return Ok(());
                }
            };
            let filename = file_stem(path)?;
            if filename.eq(project.name()) {
                let new_filename = project.id();
                let new_path = path
                    .parent()
                    .expect("projects dir should exist")
                    .join(new_filename);
                std::fs::rename(path, new_path)?;
            }
            Ok(())
        }
    }

    #[async_trait]
    impl StateItemTrait for ProjectState {
        type Config = ProjectConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            Ok(Self { path, config })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            Ok(Self { path, config })
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_state::random_name;
    use crate::cli_state::traits::StateDirTrait;
    use crate::cli_state::traits::StateItemTrait;
    use ockam_core::compat::rand::random_string;

    #[tokio::test]
    async fn migrate_project_files_using_project_id_as_filename() {
        let project_name = random_name();
        let tmp_dir = tempfile::tempdir().unwrap();
        let project_path = tmp_dir.path().join(&project_name);
        let project_config = ProjectConfig {
            name: project_name,
            id: random_string(),
            ..Default::default()
        };
        let project = ProjectState::new(project_path.clone(), project_config).unwrap();
        assert!(project_path.exists());

        // Run migration
        let projects_state = ProjectsState::new(tmp_dir.path().to_path_buf());
        projects_state.migrate(&project_path).await.unwrap();

        // Check that the project file was renamed using the project id
        let project_path = tmp_dir.path().join(project.id());
        assert!(project_path.exists());
    }
}
