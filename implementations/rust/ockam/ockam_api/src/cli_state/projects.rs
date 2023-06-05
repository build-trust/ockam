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
    name: String,
    path: PathBuf,
    config: ProjectConfig,
}

impl ProjectState {
    pub fn name(&self) -> &str {
        &self.name
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
            services: vec![],
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
        }
    }
}

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
