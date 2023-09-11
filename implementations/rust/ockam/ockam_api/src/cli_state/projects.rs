use super::Result;
use crate::cloud::project::{OktaConfig, Project};
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use ockam::identity::Identifier;
use serde::{Deserialize, Serialize};
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
    pub fn id(&self) -> &str {
        &self.config.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub type ProjectConfig = Project;

impl From<ProjectLookup> for Project {
    fn from(lookup: ProjectLookup) -> Self {
        Self {
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
            user_roles: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectConfigCompact {
    pub id: String,
    pub name: String,
    pub identity: Option<Identifier>,
    pub access_route: String,
    pub authority_access_route: Option<String>,
    pub authority_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub okta_config: Option<OktaConfig>,
}

impl TryFrom<ProjectLookup> for ProjectConfigCompact {
    type Error = ApiError;
    fn try_from(p: ProjectLookup) -> core::result::Result<Self, Self::Error> {
        Ok(Self {
            id: p.id,
            name: p.name,
            identity: p.identity_id,
            access_route: p
                .node_route
                .map_or(
                    Err(ApiError::message("Project access route is missing")),
                    Ok,
                )?
                .to_string(),
            authority_access_route: p.authority.as_ref().map(|a| a.address().to_string()),
            authority_identity: p.authority.as_ref().map(|a| hex::encode(a.identity())),
            okta_config: p.okta.map(|o| o.into()),
        })
    }
}

impl From<Project> for ProjectConfigCompact {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            identity: p.identity,
            access_route: p.access_route,
            authority_access_route: p.authority_access_route,
            authority_identity: p.authority_identity,
            okta_config: p.okta_config,
        }
    }
}

impl From<&ProjectConfigCompact> for Project {
    fn from(p: &ProjectConfigCompact) -> Self {
        Project {
            id: p.id.to_string(),
            name: p.name.to_string(),
            identity: p.identity.to_owned(),
            access_route: p.access_route.to_string(),
            authority_access_route: p.authority_access_route.as_ref().map(|a| a.to_string()),
            authority_identity: p.authority_identity.as_ref().map(|a| a.to_string()),
            okta_config: p.okta_config.clone(),
            ..Default::default()
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
