pub mod credentials;
pub mod identities;
pub mod projects;
pub mod traits;
pub mod trust_contexts;
pub mod vaults;

use crate::config::lookup::ProjectLookup;
use crate::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use nix::errno::Errno;
use ockam::identity::{Identities, IdentitiesVault, Identity};
use ockam_core::compat::sync::Arc;
use ockam_core::env::get_env_with_default;
use ockam_identity::LmdbStorage;
use ockam_vault::Vault;
use rand::random;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use sysinfo::{Pid, System, SystemExt};
use thiserror::Error;

pub use crate::cli_state::credentials::*;
pub use crate::cli_state::identities::*;
pub use crate::cli_state::projects::*;
pub use crate::cli_state::traits::*;
pub use crate::cli_state::trust_contexts::*;
pub use crate::cli_state::vaults::*;

type Result<T> = std::result::Result<T, CliStateError>;

#[derive(Debug, Error)]
pub enum CliStateError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
    #[error("ockam error")]
    Ockam(#[from] ockam_core::Error),
    #[error("already exists")]
    AlreadyExists,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Invalid(String),
    #[error("invalid state version {0}")]
    InvalidVersion(String),
    #[error("unknown error")]
    Unknown,
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
    pub projects: ProjectsState,
    pub credentials: CredentialsState,
    pub trust_contexts: TrustContextsState,
    pub dir: PathBuf,
}

impl CliState {
    pub fn try_default() -> Result<Self> {
        let dir = Self::default_dir()?;
        Self::new(&dir)
    }

    pub fn new(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir.join("defaults"))?;
        Ok(Self {
            vaults: VaultsState::load(dir)?,
            identities: IdentitiesState::load(dir)?,
            nodes: NodesState::new(dir)?,
            projects: ProjectsState::load(dir)?,
            credentials: CredentialsState::load(dir)?,
            trust_contexts: TrustContextsState::load(dir)?,
            dir: dir.to_path_buf(),
        })
    }

    pub fn test() -> Result<Self> {
        let tests_dir = dirs::home_dir()
            .ok_or(CliStateError::NotFound)?
            .join(".ockam")
            .join(".tests")
            .join(random_name());
        Self::new(&tests_dir)
    }

    pub fn delete(&self, force: bool) -> Result<()> {
        // Delete all nodes
        for n in self.nodes.list()? {
            let _ = n.delete(force);
        }

        let dir = &self.dir;
        for dir in &[
            &self.nodes.dir,
            self.identities.dir(),
            self.vaults.dir(),
            self.projects.dir(),
            self.credentials.dir(),
            self.trust_contexts.dir(),
            &dir.join("defaults"),
        ] {
            if dir.exists() {
                std::fs::remove_dir_all(dir)?
            };
        }

        let config_file = dir.join("config.json");
        if config_file.exists() {
            std::fs::remove_file(config_file)?;
        }

        // If the state directory is now empty, delete it.
        let is_empty = std::fs::read_dir(dir)?.next().is_none();
        if is_empty {
            std::fs::remove_dir(dir)?;
        }

        Ok(())
    }

    /// Returns the default directory for the CLI state.
    pub fn default_dir() -> Result<PathBuf> {
        Ok(get_env_with_default::<PathBuf>(
            "OCKAM_HOME",
            dirs::home_dir()
                .ok_or(CliStateError::NotFound)?
                .join(".ockam"),
        )?)
    }

    /// Returns the directory where the default objects are stored.
    fn defaults_dir(dir: &Path) -> Result<PathBuf> {
        Ok(dir.join("defaults"))
    }

    pub async fn create_vault_state(&self, vault_name: Option<String>) -> Result<VaultState> {
        let vault_state = if let Some(v) = vault_name {
            self.vaults.get(v.as_str())?
        }
        // Or get the default
        else if let Ok(v) = self.vaults.default() {
            v
        } else {
            let n = hex::encode(random::<[u8; 4]>());
            let c = VaultConfig::default();
            self.vaults.create_async(&n, c).await?
        };
        Ok(vault_state)
    }

    pub async fn create_identity_state(
        &self,
        identity_name: Option<String>,
        vault: Vault,
    ) -> Result<IdentityState> {
        if let Ok(identity) = self.identities.get_or_default(identity_name.clone()) {
            Ok(identity)
        } else {
            self.make_identity_state(vault, identity_name).await
        }
    }

    async fn make_identity_state(
        &self,
        vault: Vault,
        name: Option<String>,
    ) -> Result<IdentityState> {
        let identity = self
            .get_identities(vault)
            .await?
            .identities_creation()
            .create_identity()
            .await?;
        let identity_config = IdentityConfig::new(&identity).await;
        let identity_name = name.unwrap_or_else(|| hex::encode(random::<[u8; 4]>()));
        self.identities
            .create(identity_name.as_str(), identity_config)
    }

    pub async fn get_identities(&self, vault: Vault) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .with_identities_vault(Arc::new(vault))
            .with_identities_repository(self.identities.identities_repository().await?)
            .build())
    }

    pub async fn default_identities(&self) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .with_identities_vault(self.vaults.default()?.identities_vault().await?)
            .with_identities_repository(self.identities.identities_repository().await?)
            .build())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodesState {
    pub dir: PathBuf,
}

impl NodesState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("nodes");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Returns the path to the default node
    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(CliState::defaults_dir(self.dir.parent().expect("Should have parent"))?.join("node"))
    }

    pub fn default(&self) -> Result<NodeState> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        let name = file_stem(&path)?;
        self.get(&name)
    }

    pub fn set_default(&self, name: &str) -> Result<NodeState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(name);
            if !path.exists() {
                return Err(CliStateError::NotFound);
            }
            path
        };
        let link = self.default_path()?;
        // Remove file link if it exists
        let _ = std::fs::remove_file(&link);
        // Create link to new default node
        std::os::unix::fs::symlink(original, link)?;
        self.get(name)
    }

    pub fn update(&self, name: &str, mut config: NodeConfig) -> Result<NodeState> {
        config.name = name.to_string();

        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            path
        };

        let state = NodeState::new(path, config);
        std::fs::write(state.path.join("version"), state.config.version.to_string())?;
        state.set_setup(&state.config.setup)?;

        Ok(state)
    }

    pub fn create(&self, name: &str, mut config: NodeConfig) -> Result<NodeState> {
        config.name = name.to_string();

        // check if node name already exists
        if self.get_node_path(name).exists() {
            return Err(CliStateError::AlreadyExists);
        }

        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            std::fs::create_dir_all(&path)?;
            path
        };
        let state = NodeState::new(path, config);
        std::fs::write(state.path.join("version"), state.config.version.to_string())?;
        state.set_setup(&state.config.setup)?;
        std::fs::File::create(state.socket())?;
        std::fs::File::create(state.stdout_log())?;
        std::fs::File::create(state.stderr_log())?;
        std::os::unix::fs::symlink(
            &state.config.default_vault,
            state.path.join("default_vault"),
        )?;
        std::os::unix::fs::symlink(
            &state.config.default_identity,
            state.path.join("default_identity"),
        )?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }

    pub fn list(&self) -> Result<Vec<NodeState>> {
        let mut nodes = vec![];
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().into_string().map_err(|_| {
                    CliStateError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "node's directory has an invalid name",
                    ))
                })?;
                nodes.push(self.get(&name)?);
            }
        }
        Ok(nodes)
    }

    pub fn exists(&self, name: &str) -> Result<bool> {
        match self.get(name) {
            Ok(_) => Ok(true),
            Err(CliStateError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn get(&self, name: &str) -> Result<NodeState> {
        let path = self.get_node_path(name);
        if !path.exists() {
            return Err(CliStateError::NotFound);
        }

        let config = NodeConfig::try_from(&path)?;
        Ok(NodeState::new(path, config))
    }

    pub fn get_node_path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn delete(&self, name: &str, sigkill: bool) -> Result<()> {
        // Retrieve node. If doesn't exist do nothing.
        let node = match self.get(name) {
            Ok(node) => node,
            Err(CliStateError::NotFound) => return Ok(()),
            Err(e) => return Err(e),
        };

        // Set default to another node if it's the default
        if let Ok(default) = self.default() {
            if default.path == node.path {
                let _ = std::fs::remove_file(self.default_path()?);
                let mut nodes = self.list()?;
                nodes.retain(|n| n.path != node.path);
                if let Some(node) = nodes.first() {
                    self.set_default(&node.config.name)?;
                }
            }
        }

        // Remove node directory
        node.delete(sigkill)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeState {
    pub path: PathBuf,
    pub config: NodeConfig,
}

impl NodeState {
    fn new(path: PathBuf, config: NodeConfig) -> Self {
        Self { path, config }
    }

    pub fn socket(&self) -> PathBuf {
        self.path.join("socket")
    }

    pub fn setup(&self) -> Result<NodeSetupConfig> {
        NodeSetupConfig::try_from(&self.path.join("setup.json"))
    }

    pub fn set_setup(&self, setup: &NodeSetupConfig) -> Result<()> {
        let contents = serde_json::to_string(setup)?;
        std::fs::write(self.path.join("setup.json"), contents)?;
        Ok(())
    }

    pub fn pid(&self) -> Result<Option<i32>> {
        let path = self.path.join("pid");
        if self.path.join("pid").exists() {
            let pid = std::fs::read_to_string(path)?
                .parse::<i32>()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(Some(pid))
        } else {
            Ok(None)
        }
    }

    pub fn set_pid(&self, pid: i32) -> Result<()> {
        std::fs::write(self.path.join("pid"), pid.to_string())?;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        if let Ok(Some(pid)) = self.pid() {
            let mut sys = System::new();
            sys.refresh_processes();
            sys.process(Pid::from(pid as usize)).is_some()
        } else {
            false
        }
    }

    pub fn stdout_log(&self) -> PathBuf {
        self.path.join("stdout.log")
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.path.join("stderr.log")
    }

    pub async fn policies_storage(&self) -> Result<LmdbStorage> {
        Ok(LmdbStorage::new(self.path.join("policies_storage.lmdb")).await?)
    }

    pub fn kill_process(&self, sigkill: bool) -> Result<()> {
        if let Some(pid) = self.pid()? {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid),
                if sigkill {
                    nix::sys::signal::Signal::SIGKILL
                } else {
                    nix::sys::signal::Signal::SIGTERM
                },
            )
            .or_else(|e| {
                if e == Errno::ESRCH {
                    tracing::warn!(node = %self.config.name, %pid, "No such process");
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .map_err(|e| {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to stop PID `{pid}` with error `{e}`"),
                ))
            })?;
            std::fs::remove_file(self.path.join("pid"))?;
        }
        Ok(())
    }

    fn delete(&self, sigkill: bool) -> Result<()> {
        self.kill_process(sigkill)?;
        std::fs::remove_dir_all(&self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeConfig {
    pub name: String,
    version: NodeConfigVersion,
    default_vault: PathBuf,
    default_identity: PathBuf,
    setup: NodeSetupConfig,
    // TODO
    // authorities: AuthoritiesConfig,
}

impl NodeConfig {
    pub fn try_default() -> Result<Self> {
        let cli_state = CliState::try_default()?;
        Ok(Self {
            name: random_name(),
            version: NodeConfigVersion::latest(),
            default_vault: cli_state.vaults.default()?.path().clone(),
            default_identity: cli_state.identities.default()?.path().clone(),
            setup: NodeSetupConfig::default(),
        })
    }

    pub fn setup(&mut self) -> &mut NodeSetupConfig {
        &mut self.setup
    }

    pub async fn vault(&self) -> Result<Vault> {
        let state_path = std::fs::canonicalize(&self.default_vault)?;
        let state = VaultState::load(state_path)?;
        state.get().await
    }

    pub fn identity_config(&self) -> Result<IdentityConfig> {
        let path = std::fs::canonicalize(&self.default_identity)?;
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub async fn default_identity(&self) -> Result<Identity> {
        let vault: Arc<dyn IdentitiesVault> = Arc::new(self.vault().await?);
        let state_path = std::fs::canonicalize(&self.default_identity)?;
        let state = IdentityState::load(state_path)?;
        state.get(vault).await
    }
}

impl TryFrom<&CliState> for NodeConfig {
    type Error = CliStateError;

    fn try_from(cli_state: &CliState) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: random_name(),
            version: NodeConfigVersion::latest(),
            default_vault: cli_state.vaults.default()?.path().clone(),
            default_identity: cli_state.identities.default()?.path().clone(),
            setup: NodeSetupConfig::default(),
        })
    }
}

impl TryFrom<&PathBuf> for NodeConfig {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let name = {
            let err = || {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "node's directory has an invalid name",
                ))
            };
            path.file_name()
                .ok_or_else(err)?
                .to_str()
                .ok_or_else(err)?
                .to_string()
        };
        Ok(Self {
            name,
            version: NodeConfigVersion::load(path)?,
            default_vault: std::fs::canonicalize(path.join("default_vault"))?,
            default_identity: std::fs::canonicalize(path.join("default_identity"))?,
            setup: NodeSetupConfig::try_from(&path.join("setup.json"))?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeConfigBuilder {
    vault: Option<PathBuf>,
    identity: Option<PathBuf>,
}

impl NodeConfigBuilder {
    pub fn vault(mut self, path: PathBuf) -> Self {
        self.vault = Some(path);
        self
    }

    pub fn identity(mut self, path: PathBuf) -> Self {
        self.identity = Some(path);
        self
    }

    pub fn build(self, cli_state: &CliState) -> Result<NodeConfig> {
        let vault = match self.vault {
            Some(path) => path,
            None => cli_state.vaults.default()?.path().clone(),
        };
        let identity = match self.identity {
            Some(path) => path,
            None => cli_state.identities.default()?.path().clone(),
        };
        Ok(NodeConfig {
            default_vault: vault,
            default_identity: identity,
            ..NodeConfig::try_default()?
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeConfigVersion {
    V1,
}

impl NodeConfigVersion {
    fn latest() -> Self {
        Self::V1
    }

    fn load(_path: &Path) -> Result<Self> {
        Ok(Self::V1)
    }
}

impl Display for NodeConfigVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NodeConfigVersion::V1 => "1",
        })
    }
}

impl FromStr for NodeConfigVersion {
    type Err = CliStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::V1),
            _ => Err(CliStateError::InvalidVersion(s.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct NodeSetupConfig {
    pub verbose: u8,

    /// This flag is used to determine how the node status should be
    /// displayed in print_query_status.
    /// The field might be missing in previous configuration files, hence it is an Option
    pub authority_node: Option<bool>,
    pub project: Option<ProjectLookup>,
    transports: Vec<CreateTransportJson>,
    // TODO
    // secure_channels: ?,
    // inlets: ?,
    // outlets: ?,
    // services: ?,
}

impl NodeSetupConfig {
    pub fn set_verbose(mut self, verbose: u8) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn set_authority_node(mut self) -> Self {
        self.authority_node = Some(true);
        self
    }

    pub fn set_project(&mut self, project: ProjectLookup) -> &mut Self {
        self.project = Some(project);
        self
    }

    pub fn default_tcp_listener(&self) -> Result<&CreateTransportJson> {
        self.transports
            .iter()
            .find(|t| t.tt == TransportType::Tcp && t.tm == TransportMode::Listen)
            .ok_or(CliStateError::NotFound)
    }

    pub fn add_transport(mut self, transport: CreateTransportJson) -> Self {
        self.transports.push(transport);
        self
    }
}

impl TryFrom<&PathBuf> for NodeSetupConfig {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }
}

pub fn random_name() -> String {
    hex::encode(random::<[u8; 4]>())
}

fn file_stem(path: &Path) -> Result<String> {
    path.file_stem()
        .ok_or(CliStateError::NotFound)?
        .to_str()
        .map(|name| name.to_string())
        .ok_or(CliStateError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::cli::TrustContextConfig;

    #[tokio::test]
    async fn test_create_default_identity_state() {
        let state = CliState::test().unwrap();
        let vault = Vault::new(None);
        let identity1 = state
            .create_identity_state(None, vault.clone())
            .await
            .unwrap();
        let identity2 = state.create_identity_state(None, vault).await.unwrap();

        let default_identity = state.identities.default().unwrap();
        assert_eq!(identity1, default_identity);

        // make sure that a default identity is not recreated twice
        assert_eq!(identity1.name(), identity2.name());
        assert_eq!(identity1.path(), identity2.path());
    }

    #[tokio::test]
    async fn test_create_named_identity_state() {
        let state = CliState::test().unwrap();
        let vault = Vault::new(None);
        let identity1 = state
            .create_identity_state(Some("alice".into()), vault.clone())
            .await
            .unwrap();
        let identity2 = state
            .create_identity_state(Some("alice".into()), vault)
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

    // This tests way too many different things
    #[ockam_macros::test(crate = "ockam")]
    async fn integration(ctx: &mut ockam::Context) -> ockam::Result<()> {
        let sut = CliState::test()?;

        // Vaults
        let vault_name = {
            let name = hex::encode(random::<[u8; 4]>());
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
            let name = hex::encode(random::<[u8; 4]>());
            let vault_state = sut.vaults.get(&vault_name).unwrap();
            let vault: Arc<dyn IdentitiesVault> = Arc::new(vault_state.get().await.unwrap());
            let identities = Identities::builder()
                .with_identities_vault(vault)
                .with_identities_repository(sut.identities.identities_repository().await?)
                .build();
            let identity = identities
                .identities_creation()
                .create_identity()
                .await
                .unwrap();
            let config = IdentityConfig::new(&identity).await;

            let state = sut.identities.create(&name, config).unwrap();
            let got = sut.identities.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.identities.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Nodes
        let node_name = {
            let name = hex::encode(random::<[u8; 4]>());
            let config = NodeConfig::try_from(&sut).unwrap();

            let state = sut.nodes.create(&name, config).unwrap();
            let got = sut.nodes.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.nodes.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Projects
        let project_name = {
            let name = hex::encode(random::<[u8; 4]>());
            let config = ProjectConfig::default();

            let state = sut.projects.create(&name, config).unwrap();
            let got = sut.projects.get(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Trust
        let trust_context_name = {
            let name = hex::encode(random::<[u8; 4]>());
            let config = TrustContextConfig::new(name.to_string(), None);

            let state = sut.trust_contexts.create(&name, config).unwrap();
            let got = sut.trust_contexts.get(&name).unwrap();
            assert_eq!(got, state);

            name
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
            "projects".to_string(),
            format!("projects/{project_name}.json"),
            "trust_contexts".to_string(),
            format!("trust_contexts/{trust_context_name}.json"),
            "credentials".to_string(),
            "defaults".to_string(),
            "defaults/vault".to_string(),
            "defaults/identity".to_string(),
            "defaults/node".to_string(),
            "defaults/project".to_string(),
            "defaults/trust_context".to_string(),
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
                            assert_eq!(entry_name, "data");
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
                            assert_eq!(entry_name, "data");
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
                "defaults" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        let entry_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{entry_name}"));
                    });
                }
                "projects" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_file());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                "credentials" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_dir());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                "trust_contexts" => {
                    assert!(entry.path().is_dir());
                    found_entries.push(dir_name.clone());
                    entry.path().read_dir().unwrap().for_each(|entry| {
                        let entry = entry.unwrap();
                        assert!(entry.path().is_file());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                _ => panic!("unexpected file"),
            }
        });
        found_entries.sort();
        assert_eq!(expected_entries, found_entries);

        sut.projects.delete(&project_name).unwrap();
        sut.nodes.delete(&node_name, false).unwrap();
        sut.identities.delete(&identity_name).unwrap();
        sut.vaults.delete(&vault_name).unwrap();

        ctx.stop().await?;
        Ok(())
    }
}
