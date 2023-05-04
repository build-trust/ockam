use super::Result;
use crate::cli_state::{
    CliState, CliStateError, IdentityConfig, IdentityState, StateDirTrait, StateItemTrait,
    VaultState,
};
use crate::config::lookup::ProjectLookup;
use crate::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use nix::errno::Errno;
use ockam_identity::{IdentitiesVault, Identity, LmdbStorage};
use ockam_vault::Vault;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use sysinfo::{Pid, System, SystemExt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodesState {
    dir: PathBuf,
}

impl NodesState {
    pub fn delete_sigkill(&self, name: &str, sigkill: bool) -> Result<()> {
        self._delete(name, sigkill)
    }

    fn _delete(&self, name: &str, sigkill: bool) -> Result<()> {
        // If doesn't exist do nothing
        if !self.exists(name) {
            return Ok(());
        }
        let node = self.get(name)?;
        // Set default to another node if it's the default
        if self.is_default(name)? {
            // Remove link if it exists
            let _ = std::fs::remove_file(self.default_path()?);
            for node in self.list()? {
                if node.name() != name && self.set_default(node.name()).is_ok() {
                    debug!(name=%node.name(), "set default node");
                    break;
                }
            }
        }
        // Remove node directory
        node.delete_sigkill(sigkill)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeState {
    name: String,
    path: PathBuf,
    paths: NodePaths,
    config: NodeConfig,
}

impl NodeState {
    fn init(path: PathBuf, config: NodeConfig) -> Result<Self> {
        std::fs::create_dir_all(&path)?;
        let state = Self::new(path, config)?;
        std::fs::File::create(state.stdout_log())?;
        std::fs::File::create(state.stderr_log())?;
        Ok(state)
    }

    fn _delete(&self, sikgill: bool) -> Result<()> {
        self.kill_process(sikgill)?;
        std::fs::remove_dir_all(&self.path)?;
        let _ = std::fs::remove_dir(&self.path); // Make sure the dir is gone
        info!(name=%self.name, "node deleted");
        Ok(())
    }

    pub fn delete_sigkill(&self, sigkill: bool) -> Result<()> {
        self._delete(sigkill)
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
                    tracing::warn!(node = %self.name(), %pid, "No such process");
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
            std::fs::remove_file(self.paths.pid())?;
        }
        info!(name = %self.name(), "node process killed");
        Ok(())
    }

    pub fn set_setup(&self, setup: &NodeSetupConfig) -> Result<()> {
        let contents = serde_json::to_string(setup)?;
        std::fs::write(self.paths.setup(), contents)?;
        info!(name = %self.name(), "setup config updated");
        Ok(())
    }

    pub fn pid(&self) -> Result<Option<i32>> {
        let path = self.paths.pid();
        if path.exists() {
            let pid = std::fs::read_to_string(path)?
                .parse::<i32>()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(Some(pid))
        } else {
            Ok(None)
        }
    }

    pub fn set_pid(&self, pid: i32) -> Result<()> {
        std::fs::write(self.paths.pid(), pid.to_string())?;
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
        self.paths.stdout()
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.paths.stderr()
    }

    pub async fn policies_storage(&self) -> Result<LmdbStorage> {
        Ok(LmdbStorage::new(self.paths.policies_storage()).await?)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(flatten)]
    setup: NodeSetupConfig,
    #[serde(skip)]
    version: ConfigVersion,
    #[serde(skip)]
    default_vault: PathBuf,
    #[serde(skip)]
    default_identity: PathBuf,
    // TODO
    // authorities: AuthoritiesConfig,
}

impl NodeConfig {
    pub fn try_default() -> Result<Self> {
        let cli_state = CliState::try_default()?;
        Self::try_from(&cli_state)
    }

    pub fn setup(&self) -> &NodeSetupConfig {
        &self.setup
    }

    pub fn setup_mut(&self) -> NodeSetupConfig {
        self.setup.clone()
    }

    pub fn vault_path(&self) -> Result<PathBuf> {
        Ok(std::fs::canonicalize(&self.default_vault)?)
    }

    pub async fn vault(&self) -> Result<Arc<Vault>> {
        let state = VaultState::load(self.vault_path()?)?;
        state.get().await
    }

    pub fn identity_config(&self) -> Result<IdentityConfig> {
        let path = std::fs::canonicalize(&self.default_identity)?;
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub async fn identity(&self) -> Result<Identity> {
        let vault: Arc<dyn IdentitiesVault> = self.vault().await?;
        let state_path = std::fs::canonicalize(&self.default_identity)?;
        let state = IdentityState::load(state_path)?;
        state.get(vault).await
    }
}

impl TryFrom<&CliState> for NodeConfig {
    type Error = CliStateError;

    fn try_from(cli_state: &CliState) -> std::result::Result<Self, Self::Error> {
        let default_vault = cli_state.vaults.default_path()?;
        assert!(default_vault.exists(), "default vault does not exist");
        let default_identity = cli_state.identities.default_path()?;
        assert!(default_identity.exists(), "default identity does not exist");
        Ok(Self {
            version: ConfigVersion::latest(),
            default_vault,
            default_identity,
            setup: NodeSetupConfig::default(),
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
            None => cli_state.vaults.default_path()?,
        };
        let identity = match self.identity {
            Some(path) => path,
            None => cli_state.identities.default_path()?,
        };
        Ok(NodeConfig {
            default_vault: vault,
            default_identity: identity,
            ..NodeConfig::try_default()?
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigVersion {
    V1,
}

impl ConfigVersion {
    fn latest() -> Self {
        Self::V1
    }
}

impl Display for ConfigVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ConfigVersion::V1 => "1",
        })
    }
}

impl FromStr for ConfigVersion {
    type Err = CliStateError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::V1),
            _ => Err(CliStateError::InvalidVersion(s.to_string())),
        }
    }
}

impl Default for ConfigVersion {
    fn default() -> Self {
        Self::latest()
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

#[derive(Debug, Clone, Eq, PartialEq)]
struct NodePaths {
    path: PathBuf,
}

impl NodePaths {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    fn setup(&self) -> PathBuf {
        self.path.join("setup.json")
    }

    fn vault(&self) -> PathBuf {
        self.path.join("default_vault")
    }

    fn identity(&self) -> PathBuf {
        self.path.join("default_identity")
    }

    fn pid(&self) -> PathBuf {
        self.path.join("pid")
    }

    fn version(&self) -> PathBuf {
        self.path.join("version")
    }

    fn stdout(&self) -> PathBuf {
        self.path.join("stdout.log")
    }

    fn stderr(&self) -> PathBuf {
        self.path.join("stderr.log")
    }

    fn policies_storage(&self) -> PathBuf {
        self.path.join("policies_storage.lmdb")
    }
}

mod traits {
    use super::*;
    use crate::cli_state::traits::*;
    use crate::cli_state::{file_stem, CliStateError};
    use ockam_core::async_trait;

    #[async_trait]
    impl StateDirTrait for NodesState {
        type Item = NodeState;
        const DEFAULT_FILENAME: &'static str = "node";
        const DIR_NAME: &'static str = "nodes";
        const HAS_DATA_DIR: bool = false;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn path(&self, name: &str) -> PathBuf {
            self.dir().join(name)
        }

        fn create(
            &self,
            name: &str,
            config: <<Self as StateDirTrait>::Item as StateItemTrait>::Config,
        ) -> Result<Self::Item> {
            if self.exists(name) {
                return Err(CliStateError::AlreadyExists);
            }
            let state = Self::Item::init(self.path(name), config)?;
            if !self.default_path()?.exists() {
                self.set_default(name)?;
            }
            Ok(state)
        }

        fn delete(&self, name: &str) -> Result<()> {
            self._delete(name, false)
        }
    }

    #[async_trait]
    impl StateItemTrait for NodeState {
        type Config = NodeConfig;

        fn new(path: PathBuf, mut config: Self::Config) -> Result<Self> {
            let paths = NodePaths::new(&path);
            let name = file_stem(&path)?;
            std::fs::write(paths.setup(), serde_json::to_string(config.setup())?)?;
            std::fs::write(paths.version(), config.version.to_string())?;
            let _ = std::fs::remove_file(paths.vault());
            std::os::unix::fs::symlink(&config.default_vault, paths.vault())?;
            config.default_vault = paths.vault();
            let _ = std::fs::remove_file(paths.identity());
            std::os::unix::fs::symlink(&config.default_identity, paths.identity())?;
            config.default_identity = paths.identity();
            Ok(Self {
                name,
                path,
                paths,
                config,
            })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let paths = NodePaths::new(&path);
            let name = file_stem(&path)?;
            let setup = {
                let contents = std::fs::read_to_string(paths.setup())?;
                serde_json::from_str(&contents)?
            };
            let version = {
                let contents = std::fs::read_to_string(paths.version())?;
                contents.parse::<ConfigVersion>()?
            };
            let config = NodeConfig {
                setup,
                version,
                default_vault: paths.vault(),
                default_identity: paths.identity(),
            };
            Ok(Self {
                name,
                path,
                paths,
                config,
            })
        }

        fn delete(&self) -> Result<()> {
            self._delete(false)
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
