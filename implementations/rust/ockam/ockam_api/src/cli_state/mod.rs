pub mod credentials;
pub mod identities;
pub mod nodes;
pub mod projects;
pub mod spaces;
pub mod traits;
pub mod trust_contexts;
pub mod user_info;
pub mod vaults;

pub use crate::cli_state::credentials::*;
pub use crate::cli_state::identities::*;
pub use crate::cli_state::nodes::*;
pub use crate::cli_state::projects::*;
pub use crate::cli_state::spaces::*;
pub use crate::cli_state::traits::*;
pub use crate::cli_state::trust_contexts::*;
use crate::cli_state::user_info::UsersInfoState;
pub use crate::cli_state::vaults::*;
use crate::config::cli::LegacyCliConfig;
use miette::Diagnostic;
use ockam::identity::Identifier;
use ockam::identity::Identities;
use ockam::identity::Vault;
use ockam_core::compat::sync::Arc;
use ockam_core::env::get_env_with_default;
use ockam_node::Executor;
use rand::random;
use std::path::{Path, PathBuf};
use thiserror::Error;

type Result<T> = std::result::Result<T, CliStateError>;

#[derive(Debug, Error, Diagnostic)]
pub enum CliStateError {
    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code("OCK500"))]
    Ockam(#[from] ockam_core::Error),

    #[error("A {resource} named {name} already exists")]
    #[diagnostic(
        code("OCK409"),
        help("Please try using a different name or delete the existing {resource}")
    )]
    AlreadyExists { resource: String, name: String },

    #[error("Unable to find {resource} named {name}")]
    #[diagnostic(code("OCK404"))]
    ResourceNotFound { resource: String, name: String },

    #[error("The path {0} is invalid")]
    #[diagnostic(code("OCK500"))]
    InvalidPath(String),

    #[error("The path is empty")]
    #[diagnostic(code("OCK500"))]
    EmptyPath,

    #[error("{0}")]
    #[diagnostic(code("OCK500"))]
    InvalidData(String),

    #[error("{0}")]
    #[diagnostic(code("OCK500"))]
    InvalidOperation(String),

    #[error("Invalid configuration version '{0}'")]
    #[diagnostic(
        code("OCK500"),
        help("Please try running 'ockam reset' to reset your local configuration")
    )]
    InvalidVersion(String),
}

impl From<&str> for CliStateError {
    fn from(e: &str) -> Self {
        CliStateError::InvalidOperation(e.to_string())
    }
}

impl From<CliStateError> for ockam_core::Error {
    fn from(e: CliStateError) -> Self {
        match e {
            CliStateError::Ockam(e) => e,
            _ => ockam_core::Error::new(
                ockam_core::errcode::Origin::Application,
                ockam_core::errcode::Kind::Internal,
                e,
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CliState {
    pub vaults: VaultsState,
    pub identities: IdentitiesState,
    pub nodes: NodesState,
    pub spaces: SpacesState,
    pub projects: ProjectsState,
    pub credentials: CredentialsState,
    pub trust_contexts: TrustContextsState,
    pub users_info: UsersInfoState,
    pub dir: PathBuf,
}

impl CliState {
    /// Return an initialized CliState
    /// There should only be one call to this function since it also performs a migration
    /// of configuration files if necessary
    pub fn initialize() -> Result<Self> {
        let dir = Self::default_dir()?;
        std::fs::create_dir_all(dir.join("defaults"))?;
        Executor::execute_future(Self::initialize_cli_state())?
    }

    /// Create a new CliState by initializing all of its components
    /// The calls to 'init(dir)' are loading each piece of configuration and possibly doing some
    /// configuration migration if necessary
    async fn initialize_cli_state() -> Result<CliState> {
        let default = Self::default_dir()?;
        let dir = default.as_path();
        let state = Self {
            vaults: VaultsState::init(dir).await?,
            identities: IdentitiesState::init(dir).await?,
            nodes: NodesState::init(dir).await?,
            spaces: SpacesState::init(dir).await?,
            projects: ProjectsState::init(dir).await?,
            credentials: CredentialsState::init(dir).await?,
            trust_contexts: TrustContextsState::init(dir).await?,
            users_info: UsersInfoState::init(dir).await?,
            dir: dir.to_path_buf(),
        };
        state.migrate()?;
        Ok(state)
    }

    /// Reset all directories and return a new CliState
    pub async fn reset(&self) -> Result<CliState> {
        Self::delete_at(&self.dir)?;
        Self::initialize_cli_state().await
    }

    pub fn backup_and_reset() -> Result<CliState> {
        let dir = Self::default_dir()?;

        // Reset backup directory
        let backup_dir = Self::backup_default_dir()?;
        if backup_dir.exists() {
            let _ = std::fs::remove_dir_all(&backup_dir);
        }
        std::fs::create_dir_all(&backup_dir)?;

        // Move state to backup directory
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let from = entry.path();
            let to = backup_dir.join(entry.file_name());
            std::fs::rename(from, to)?;
        }

        // Reset state
        Self::delete_at(&dir)?;
        Self::initialize()
    }

    fn migrate(&self) -> Result<()> {
        // If there is a `config.json` file, migrate its contents to the spaces and project states.
        let legacy_config_path = self.dir.join("config.json");
        if legacy_config_path.exists() {
            let contents = std::fs::read_to_string(&legacy_config_path)?;
            let legacy_config: LegacyCliConfig = serde_json::from_str(&contents)?;
            let spaces = self.spaces.list()?;
            for (name, lookup) in legacy_config.lookup.spaces() {
                if !spaces.iter().any(|s| s.name() == name) {
                    let config = SpaceConfig::from_lookup(&name, lookup);
                    self.spaces.create(name, config)?;
                }
            }
            let projects = self.projects.list()?;
            for (name, lookup) in legacy_config.lookup.projects() {
                if !projects.iter().any(|p| p.name() == name) {
                    self.projects.create(name, lookup.into())?;
                }
            }
            std::fs::remove_file(legacy_config_path)?;
        }
        Ok(())
    }

    pub fn delete_at(root_path: &PathBuf) -> Result<()> {
        // Delete nodes' state and processes, if possible
        let nodes_state = NodesState::new(root_path);
        let _ = nodes_state.list().map(|nodes| {
            nodes.iter().for_each(|n| {
                let _ = n.delete_sigkill(true);
            });
        });

        // Delete all other state directories
        for dir in &[
            nodes_state.dir(),
            IdentitiesState::new(root_path).dir(),
            VaultsState::new(root_path).dir(),
            SpacesState::new(root_path).dir(),
            ProjectsState::new(root_path).dir(),
            CredentialsState::new(root_path).dir(),
            TrustContextsState::new(root_path).dir(),
            UsersInfoState::new(root_path).dir(),
            &root_path.join("defaults"),
        ] {
            let _ = std::fs::remove_dir_all(dir);
        }

        // Delete config files located at the root of the state directory
        let config_file = root_path.join("config.json");
        let _ = std::fs::remove_file(config_file);

        // If the state directory is now empty, delete it
        let is_empty = std::fs::read_dir(root_path)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(root_path);
        }

        Ok(())
    }

    pub fn delete() -> Result<()> {
        Self::delete_at(&Self::default_dir()?)
    }

    pub fn delete_identity(&self, identity_state: IdentityState) -> Result<()> {
        // Abort if identity is being used by some running node.
        for node in self.nodes.list()? {
            if node.config().identity_config()?.identifier() == identity_state.identifier() {
                return Err(CliStateError::InvalidOperation(format!(
                    "Can't delete identity '{}' as it's being used by node '{}'",
                    &identity_state.name(),
                    &node.name()
                )));
            }
        }
        identity_state.delete()
    }

    /// Returns the default directory for the CLI state.
    pub fn default_dir() -> Result<PathBuf> {
        Ok(get_env_with_default::<PathBuf>(
            "OCKAM_HOME",
            home::home_dir()
                .ok_or(CliStateError::InvalidPath("$HOME".to_string()))?
                .join(".ockam"),
        )?)
    }

    /// Returns the default backup directory for the CLI state.
    pub fn backup_default_dir() -> Result<PathBuf> {
        let dir = Self::default_dir()?;
        let dir_name =
            dir.file_name()
                .and_then(|n| n.to_str())
                .ok_or(CliStateError::InvalidOperation(
                    "The $OCKAM_HOME directory does not have a valid name".to_string(),
                ))?;
        let parent = dir.parent().ok_or(CliStateError::InvalidOperation(
            "The $OCKAM_HOME directory does not a valid parent directory".to_string(),
        ))?;
        Ok(parent.join(format!("{dir_name}.bak")))
    }

    /// Returns the directory where the default objects are stored.
    fn defaults_dir(dir: &Path) -> Result<PathBuf> {
        Ok(dir.join("defaults"))
    }

    pub async fn create_vault_state(&self, vault_name: Option<&str>) -> Result<VaultState> {
        // Try to get the vault with the given name
        let vault_state = if let Some(v) = vault_name {
            self.vaults.get(v)?
        }
        // Or get the default
        else if let Ok(v) = self.vaults.default() {
            v
        }
        // Or create a new one with a random name
        else {
            let n = random_name();
            let c = VaultConfig::default();
            self.vaults.create_async(&n, c).await?
        };
        Ok(vault_state)
    }

    pub async fn create_identity_state(
        &self,
        identifier: &Identifier,
        identity_name: Option<&str>,
    ) -> Result<IdentityState> {
        if let Ok(identity) = self.identities.get_or_default(identity_name) {
            Ok(identity)
        } else {
            self.make_identity_state(identifier, identity_name).await
        }
    }

    async fn make_identity_state(
        &self,
        identifier: &Identifier,
        name: Option<&str>,
    ) -> Result<IdentityState> {
        let identity_config = IdentityConfig::new(identifier).await;
        let identity_name = name.map(|x| x.to_string()).unwrap_or_else(random_name);
        self.identities.create(identity_name, identity_config)
    }

    pub async fn get_identities(&self, vault: Vault) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .with_vault(vault)
            .with_identities_repository(self.identities.identities_repository().await?)
            .build())
    }

    pub async fn default_identities(&self) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .with_vault(self.vaults.default()?.vault().await?)
            .with_identities_repository(self.identities.identities_repository().await?)
            .build())
    }

    /// Return true if the user is enrolled.
    /// At the moment this check only verifies that there is a default project.
    /// This project should be the project that is created at the end of the enrollment procedure
    pub fn is_enrolled(&self) -> Result<bool> {
        let identity_state = self.identities.default()?;
        if !identity_state.is_enrolled() {
            return Ok(false);
        }

        let default_space_exists = self.spaces.default().is_ok();
        if !default_space_exists {
            let message =
                "There should be a default space set for the current user. Please re-enroll";
            error!("{}", message);
            return Err(message.into());
        }

        let default_project_exists = self.projects.default().is_ok();
        if !default_project_exists {
            let message =
                "There should be a default project set for the current user. Please re-enroll";
            error!("{}", message);
            return Err(message.into());
        }

        Ok(true)
    }
}

/// Test support
impl CliState {
    #[cfg(test)]
    /// Initialize CliState at the given directory
    async fn initialize_at(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir.join("defaults"))?;
        let state = Self {
            vaults: VaultsState::init(dir).await?,
            identities: IdentitiesState::init(dir).await?,
            nodes: NodesState::init(dir).await?,
            spaces: SpacesState::init(dir).await?,
            projects: ProjectsState::init(dir).await?,
            credentials: CredentialsState::init(dir).await?,
            trust_contexts: TrustContextsState::init(dir).await?,
            users_info: UsersInfoState::init(dir).await?,
            dir: dir.to_path_buf(),
        };
        state.migrate()?;
        Ok(state)
    }

    /// Create a new CliState (but do not run migrations)
    fn new(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir.join("defaults"))?;
        Ok(Self {
            vaults: VaultsState::load(dir)?,
            identities: IdentitiesState::load(dir)?,
            nodes: NodesState::load(dir)?,
            spaces: SpacesState::load(dir)?,
            projects: ProjectsState::load(dir)?,
            credentials: CredentialsState::load(dir)?,
            trust_contexts: TrustContextsState::load(dir)?,
            users_info: UsersInfoState::load(dir)?,
            dir: dir.to_path_buf(),
        })
    }

    /// Return a test CliState with a random root directory
    pub fn test() -> Result<Self> {
        Self::new(&Self::test_dir()?)
    }

    /// Return a random root directory
    pub fn test_dir() -> Result<PathBuf> {
        Ok(home::home_dir()
            .ok_or(CliStateError::InvalidPath("$HOME".to_string()))?
            .join(".ockam")
            .join(".tests")
            .join(random_name()))
    }
}

pub fn random_name() -> String {
    petname::petname(2, "-").unwrap_or(hex::encode(random::<[u8; 4]>()))
}

fn file_stem(path: &Path) -> Result<String> {
    let path_str = path.to_str().ok_or(CliStateError::EmptyPath)?;
    path.file_stem()
        .ok_or(CliStateError::InvalidPath(path_str.to_string()))?
        .to_str()
        .map(|name| name.to_string())
        .ok_or(CliStateError::InvalidPath(path_str.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::enroll::auth0::UserInfo;
    use crate::config::cli::TrustContextConfig;
    use crate::config::lookup::{ConfigLookup, LookupValue, ProjectLookup, SpaceLookup};
    use ockam_core::compat::rand::random_string;
    use ockam_multiaddr::MultiAddr;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_create_default_identity_state() {
        let state = CliState::test().unwrap();
        let identifier = "Ie92f183eb4c324804ef4d62962dea94cf095a265"
            .try_into()
            .unwrap();
        let identity1 = state
            .create_identity_state(&identifier, None)
            .await
            .unwrap();
        let identity2 = state
            .create_identity_state(&identifier, None)
            .await
            .unwrap();

        let default_identity = state.identities.default().unwrap();
        assert_eq!(identity1, default_identity);

        // make sure that a default identity is not recreated twice
        assert_eq!(identity1.name(), identity2.name());
        assert_eq!(identity1.path(), identity2.path());
    }

    #[tokio::test]
    async fn test_create_named_identity_state() {
        let state = CliState::test().unwrap();
        let alice = "Ie92f183eb4c324804ef4d62962dea94cf095a265"
            .try_into()
            .unwrap();
        let identity1 = state
            .create_identity_state(&alice, Some("alice"))
            .await
            .unwrap();
        let identity2 = state
            .create_identity_state(&alice, Some("alice"))
            .await
            .unwrap();

        assert_eq!(identity1.name(), "alice");
        assert!(identity1
            .path()
            .to_string_lossy()
            .to_string()
            .contains("alice.json"));

        // make sure that a named identity is not recreated twice
        assert_eq!(identity1.name(), identity2.name());
        assert_eq!(identity1.path(), identity2.path());
    }

    #[tokio::test]
    async fn migrate_legacy_cli_config() {
        // Before this migration, there was a `config.json` file in the root $OCKAM_HOME directory
        // that contained a map of space names to space and project lookups. This test ensures that
        // the migration correctly moves the space and project lookups into the new `spaces` and
        // `projects` directories, respectively.
        let space_name = "sname";
        let space_lookup = SpaceLookup {
            id: "sid".to_string(),
        };
        let project_lookup = ProjectLookup {
            node_route: Some(MultiAddr::from_str("/node/p").unwrap()),
            id: "pid".to_string(),
            name: "pname".to_string(),
            identity_id: Some(
                Identifier::from_str("Ibb37445cacb3ca7a20040a9b36469e321a57d2cd").unwrap(),
            ),
            authority: None,
            okta: None,
        };
        let test_dir = CliState::test_dir().unwrap();
        let legacy_config = {
            let map = vec![
                (space_name.to_string(), LookupValue::Space(space_lookup)),
                (
                    project_lookup.name.clone(),
                    LookupValue::Project(project_lookup.clone()),
                ),
            ];
            let lookup = ConfigLookup {
                map: map.into_iter().collect(),
            };
            LegacyCliConfig {
                dir: Some(test_dir.clone()),
                lookup,
            }
        };
        std::fs::create_dir_all(&test_dir).unwrap();
        std::fs::write(
            test_dir.join("config.json"),
            serde_json::to_string(&legacy_config).unwrap(),
        )
        .unwrap();
        let state = CliState::initialize_at(&test_dir).await.unwrap();
        let space = state.spaces.get(space_name).unwrap();
        assert_eq!(space.config().id, "sid");
        let project = state.projects.get(&project_lookup.name).unwrap();
        assert_eq!(project.config().id, project_lookup.id);
        assert_eq!(
            project.config().access_route,
            project_lookup.node_route.unwrap().to_string()
        );
        assert!(!test_dir.join("config.json").exists());
    }

    #[ockam_macros::test(crate = "ockam")]
    async fn integration(ctx: &mut ockam::Context) -> ockam::Result<()> {
        let sut = CliState::test()?;

        // Vaults
        let vault_name = {
            let name = random_name();
            let config = VaultConfig::default();

            let state = sut.vaults.create_async(&name, config).await.unwrap();
            let got = sut.vaults.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.vaults.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Identities
        let identity_name = {
            let name = random_name();
            let vault_state = sut.vaults.get(&vault_name).unwrap();
            let vault: Vault = vault_state.get().await.unwrap();
            let identities = Identities::builder()
                .with_vault(vault)
                .with_identities_repository(sut.identities.identities_repository().await?)
                .build();
            let identity = identities
                .identities_creation()
                .create_identity()
                .await
                .unwrap();
            let config = IdentityConfig::new(identity.identifier()).await;

            let state = sut.identities.create(&name, config).unwrap();
            let got = sut.identities.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.identities.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Nodes
        let node_name = {
            let name = random_name();
            let config = NodeConfig::try_from(&sut).unwrap();

            let state = sut.nodes.create(&name, config).unwrap();
            let got = sut.nodes.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.nodes.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Spaces
        let space_name = {
            let name = random_name();
            let id = random_string();
            let config = SpaceConfig {
                name: name.clone(),
                id,
            };

            let state = sut.spaces.create(&name, config).unwrap();
            let got = sut.spaces.get(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Projects
        let project_name = {
            let name = random_name();
            let config = ProjectConfig::default();

            let state = sut.projects.create(&name, config).unwrap();
            let got = sut.projects.get(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Trust Contexts
        let trust_context_name = {
            let name = random_name();
            let config = TrustContextConfig::new(name.to_string(), None);

            let state = sut.trust_contexts.create(&name, config).unwrap();
            let got = sut.trust_contexts.get(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Users Info
        let user_info_email = {
            let email = random_name();
            let config = UserInfo {
                email: email.clone(),
                ..Default::default()
            };

            let state = sut.users_info.create(&email, config).unwrap();
            let got = sut.users_info.get(&email).unwrap();
            assert_eq!(got, state);

            email
        };

        // Check structure
        let mut expected_entries = vec![
            "vaults".to_string(),
            format!("vaults/{vault_name}.json"),
            "vaults/data".to_string(),
            format!("vaults/data/{vault_name}-storage.json"),
            "identities".to_string(),
            format!("identities/{identity_name}.json"),
            "identities/data/authenticated_storage.lmdb".to_string(),
            "nodes".to_string(),
            format!("nodes/{node_name}"),
            "spaces".to_string(),
            format!("spaces/{space_name}.json"),
            "projects".to_string(),
            format!("projects/{project_name}.json"),
            "trust_contexts".to_string(),
            format!("trust_contexts/{trust_context_name}.json"),
            "users_info".to_string(),
            format!("users_info/{user_info_email}.json"),
            "credentials".to_string(),
            "defaults".to_string(),
            "defaults/vault".to_string(),
            "defaults/identity".to_string(),
            "defaults/node".to_string(),
            "defaults/space".to_string(),
            "defaults/project".to_string(),
            "defaults/trust_context".to_string(),
            "defaults/user_info".to_string(),
        ];
        expected_entries.sort();
        let mut found_entries = vec![];
        sut.dir.read_dir().unwrap().for_each(|entry| {
            let entry = entry.unwrap();
            let dir_name = entry.file_name().into_string().unwrap();
            match dir_name.as_str() {
                "vaults" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        let entry_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{entry_name}"));
                        if entry.path().is_dir() {
                            assert_eq!(entry_name, DATA_DIR_NAME);
                            entry.path().read_dir().unwrap().for_each(|entry| {
                                let entry = entry.unwrap();
                                let file_name = entry.file_name().into_string().unwrap();
                                if !file_name.ends_with(".lock") {
                                    found_entries
                                        .push(format!("{dir_name}/{entry_name}/{file_name}"));
                                    assert_eq!(file_name, format!("{vault_name}-storage.json"));
                                }
                            });
                        } else {
                            assert_eq!(entry_name, format!("{vault_name}.json"));
                        }
                    });
                }
                "identities" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        let entry_name = entry.file_name().into_string().unwrap();
                        if entry.path().is_dir() {
                            assert_eq!(entry_name, DATA_DIR_NAME);
                            entry.path().read_dir().unwrap().for_each(|entry| {
                                let entry = entry.unwrap();
                                let file_name = entry.file_name().into_string().unwrap();
                                if !file_name.ends_with("-lock") {
                                    found_entries
                                        .push(format!("{dir_name}/{entry_name}/{file_name}"));
                                    assert_eq!(file_name, format!("authenticated_storage.lmdb"));
                                }
                            })
                        } else {
                            assert!(entry.path().is_file());
                            let file_name = entry.file_name().into_string().unwrap();
                            found_entries.push(format!("{dir_name}/{file_name}"));
                        }
                    });
                }
                "nodes" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_dir());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                "defaults" | "spaces" | "projects" | "credentials" | "trust_contexts"
                | "users_info" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        let entry_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{entry_name}"));
                    });
                }
                _ => panic!("unexpected file"),
            }
        });
        found_entries.sort();
        assert_eq!(expected_entries, found_entries);

        sut.spaces.delete(&space_name).unwrap();
        sut.projects.delete(&project_name).unwrap();
        sut.nodes.delete(&node_name).unwrap();
        sut.identities.delete(&identity_name).unwrap();
        sut.vaults.delete(&vault_name).unwrap();

        ctx.stop().await?;
        Ok(())
    }
}
