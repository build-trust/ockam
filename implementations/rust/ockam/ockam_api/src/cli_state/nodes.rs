use super::Result;
use crate::cli_state::{
    CliState, CliStateError, IdentityConfig, IdentityState, ProjectConfig, ProjectConfigCompact,
    StateDirTrait, StateItemTrait, VaultState,
};
use crate::config::lookup::ProjectLookup;
use crate::nodes::models::transport::CreateTransportJson;
use backwards_compatibility::*;
use miette::{IntoDiagnostic, WrapErr};
use nix::errno::Errno;
use ockam::identity::Identifier;
use ockam::identity::Vault;
use ockam::LmdbStorage;
use ockam_core::compat::collections::HashSet;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use sysinfo::{Pid, ProcessExt, ProcessStatus, System, SystemExt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodesState {
    dir: PathBuf,
}

impl NodesState {
    pub fn stdout_logs(&self, name: &str) -> Result<PathBuf> {
        let dir = self.path(name);
        std::fs::create_dir_all(&dir)?;
        Ok(NodePaths::new(&dir).stdout())
    }

    pub fn delete_sigkill(&self, name: &str, sigkill: bool) -> Result<()> {
        self._delete(name, sigkill)
    }

    fn _delete(&self, name: impl AsRef<str>, sigkill: bool) -> Result<()> {
        // If doesn't exist do nothing
        if !self.exists(&name) {
            return Ok(());
        }
        let node = self.get(&name)?;
        // Set default to another node if it's the default
        if self.is_default(&name)? {
            // Remove link if it exists
            let _ = std::fs::remove_file(self.default_path()?);
            for node in self.list()? {
                if node.name() != name.as_ref() && self.set_default(node.name()).is_ok() {
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
            if let Some(p) = sys.process(Pid::from(pid as usize)) {
                // Under certain circumstances the process can be in a state where it's not running
                // and we are unable to kill it. For example, `kill -9` a process created by
                // `node create` in a Docker environment will result in a zombie process.
                !matches!(p.status(), ProcessStatus::Dead | ProcessStatus::Zombie)
            } else {
                false
            }
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
}

impl NodeConfig {
    pub fn new(cli_state: &CliState) -> Result<Self> {
        Self::try_from(cli_state)
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

    pub async fn vault(&self) -> Result<Vault> {
        let state = VaultState::load(self.vault_path()?)?;
        state.get().await
    }

    pub fn identity_config(&self) -> Result<IdentityConfig> {
        let path = std::fs::canonicalize(&self.default_identity)?;
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn identifier(&self) -> Result<Identifier> {
        let state_path = std::fs::canonicalize(&self.default_identity)?;
        let state = IdentityState::load(state_path)?;
        Ok(state.identifier())
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
            ..NodeConfig::new(cli_state)?
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
    pub api_transport: Option<CreateTransportJson>,
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

    pub fn set_api_transport(mut self, transport: CreateTransportJson) -> Self {
        self.api_transport = Some(transport);
        self
    }

    pub fn api_transport(&self) -> Result<&CreateTransportJson> {
        self.api_transport.as_ref().ok_or_else(|| {
            CliStateError::InvalidOperation(
                "The api transport was not set for the node".to_string(),
            )
        })
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

mod backwards_compatibility {
    use super::*;

    #[derive(Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub(super) enum NodeConfigs {
        V1(NodeConfigV1),
        V2(NodeConfig),
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub(super) struct NodeConfigV1 {
        #[serde(flatten)]
        pub setup: NodeSetupConfigV1,
        #[serde(skip)]
        pub version: ConfigVersion,
        #[serde(skip)]
        pub default_vault: PathBuf,
        #[serde(skip)]
        pub default_identity: PathBuf,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub(super) enum NodeSetupConfigs {
        V1(NodeSetupConfigV1),
        V2(NodeSetupConfig),
    }

    // The change was replacing the `transports` field with `api_transport`
    #[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
    pub(super) struct NodeSetupConfigV1 {
        pub verbose: u8,

        /// This flag is used to determine how the node status should be
        /// displayed in print_query_status.
        /// The field might be missing in previous configuration files, hence it is an Option
        pub authority_node: Option<bool>,
        pub project: Option<ProjectLookup>,
        pub transports: HashSet<CreateTransportJson>,
    }

    #[cfg(test)]
    impl NodeSetupConfigV1 {
        pub fn add_transport(mut self, transport: CreateTransportJson) -> Self {
            self.transports.insert(transport);
            self
        }
    }
}

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use crate::nodes::models::transport::{TransportMode, TransportType};
    use ockam_core::async_trait;

    #[async_trait]
    impl StateDirTrait for NodesState {
        type Item = NodeState;
        const DEFAULT_FILENAME: &'static str = "node";
        const DIR_NAME: &'static str = "nodes";
        const HAS_DATA_DIR: bool = false;

        fn new(root_path: &Path) -> Self {
            Self {
                dir: Self::build_dir(root_path),
            }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn path(&self, name: impl AsRef<str>) -> PathBuf {
            self.dir().join(name.as_ref())
        }

        /// A node contains several files, and the existence of the main directory is not not enough
        /// to determine if a node exists as it could be created but empty.
        fn exists(&self, name: impl AsRef<str>) -> bool {
            let paths = NodePaths::new(&self.path(&name));
            paths.setup().exists()
        }

        fn delete(&self, name: impl AsRef<str>) -> Result<()> {
            self._delete(&name, false)
        }

        async fn migrate(&self, node_path: &Path) -> Result<()> {
            if node_path.is_file() {
                // If path is a file, it is probably a non supported file (e.g. .DS_Store)
                return Ok(());
            }
            let paths = NodePaths::new(node_path);
            let contents = std::fs::read_to_string(paths.setup())?;
            match serde_json::from_str(&contents)? {
                NodeSetupConfigs::V1(setup) => {
                    // Get the first tcp-listener from the transports hashmap and
                    // use it as the api transport
                    let mut new_setup = NodeSetupConfig {
                        verbose: setup.verbose,
                        authority_node: setup.authority_node,
                        project: setup.project,
                        api_transport: None,
                    };
                    if let Some(t) = setup
                        .transports
                        .into_iter()
                        .find(|t| t.tt == TransportType::Tcp && t.tm == TransportMode::Listen)
                    {
                        new_setup.api_transport = Some(t);
                    }
                    std::fs::write(paths.setup(), serde_json::to_string(&new_setup)?)?;
                }
                NodeSetupConfigs::V2(_) => (),
            }
            Ok(())
        }
    }

    #[async_trait]
    impl StateItemTrait for NodeState {
        type Config = NodeConfig;

        fn new(path: PathBuf, mut config: Self::Config) -> Result<Self> {
            std::fs::create_dir_all(&path)?;
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

pub async fn init_node_state(
    cli_state: &CliState,
    node_name: &str,
    vault_name: Option<&str>,
    identity_name: Option<&str>,
) -> miette::Result<()> {
    debug!(name=%node_name, "initializing node state");
    // Get vault specified in the argument, or get the default
    let vault_state = cli_state.create_vault_state(vault_name).await?;

    // create an identity for the node
    let identity = cli_state
        .get_identities(vault_state.get().await?)
        .await?
        .identities_creation()
        .create_identity()
        .await
        .into_diagnostic()
        .wrap_err("Failed to create identity")?;

    let identity_state = cli_state
        .create_identity_state(identity.identifier(), identity_name)
        .await?;

    // Create the node with the given vault and identity
    let node_config = NodeConfigBuilder::default()
        .vault(vault_state.path().clone())
        .identity(identity_state.path().clone())
        .build(cli_state)?;
    cli_state.nodes.overwrite(node_name, node_config)?;

    info!(name=%node_name, "node state initialized");
    Ok(())
}

pub async fn add_project_info_to_node_state(
    node_name: &str,
    cli_state: &CliState,
    project_path: Option<&PathBuf>,
) -> Result<Option<String>> {
    debug!(name=%node_name, "Adding project info to state");
    let proj_path = if let Some(path) = project_path {
        Some(path.clone())
    } else if let Ok(proj) = cli_state.projects.default() {
        Some(proj.path().clone())
    } else {
        None
    };

    match proj_path {
        Some(path) => {
            debug!(path=%path.display(), "Reading project info from path");
            let s = std::fs::read_to_string(path)?;
            let proj_info: ProjectConfigCompact = serde_json::from_str(&s)?;
            let proj_lookup = ProjectLookup::from_project(&(&proj_info).into())
                .await
                .map_err(|e| {
                    CliStateError::InvalidData(format!("Failed to read project: {}", e))
                })?;
            let proj_config = ProjectConfig::from(&proj_info);
            let state = cli_state.nodes.get(node_name)?;
            state.set_setup(state.config().setup_mut().set_project(proj_lookup.clone()))?;
            cli_state
                .projects
                .overwrite(proj_lookup.name, proj_config)?;
            Ok(Some(proj_lookup.id))
        }
        None => {
            debug!("No project info used");
            Ok(None)
        }
    }
}

pub async fn update_enrolled_identity(cli_state: &CliState, node_name: &str) -> Result<Identifier> {
    let identities = cli_state.identities.list()?;

    let node_state = cli_state.nodes.get(node_name)?;
    let node_identifier = node_state.config().identifier()?;

    for mut identity in identities {
        if node_identifier == identity.config().identifier() {
            identity.set_enrollment_status()?;
        }
    }

    Ok(node_identifier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::lookup::InternetAddress;
    use crate::nodes::models::transport::{TransportMode, TransportType};

    #[test]
    fn node_config_setup_transports_no_duplicates() {
        let mut config = NodeSetupConfigV1 {
            verbose: 0,
            authority_node: None,
            project: None,
            transports: HashSet::new(),
        };
        let transport = CreateTransportJson {
            tt: TransportType::Tcp,
            tm: TransportMode::Listen,
            addr: InternetAddress::V4("127.0.0.1:1020".parse().unwrap()),
        };
        config = config.add_transport(transport.clone());
        assert_eq!(config.transports.len(), 1);
        assert_eq!(config.transports.iter().next(), Some(&transport));

        config = config.add_transport(transport);
        assert_eq!(config.transports.len(), 1);
    }

    #[test]
    fn node_config_setup_transports_parses_a_json_with_duplicate_entries() {
        // This test is to ensure backwards compatibility, for versions where transports where stored as a Vec<>
        let config_json = r#"{
            "verbose": 0,
            "authority_node": null,
            "project": null,
            "transports": [
                {"tt":"Tcp","tm":"Listen","addr":{"V4":"127.0.0.1:1020"}},
                {"tt":"Tcp","tm":"Listen","addr":{"V4":"127.0.0.1:1020"}}
            ]
        }"#;
        let config = serde_json::from_str::<NodeSetupConfigV1>(config_json).unwrap();
        assert_eq!(config.transports.len(), 1);
    }

    #[tokio::test]
    async fn migrate_node_config_from_v1_to_v2() {
        // Create a v1 setup.json file
        let v1_json_json = r#"{
            "verbose": 0,
            "authority_node": null,
            "project": null,
            "transports": [
                {"tt":"Tcp","tm":"Listen","addr":{"V4":"127.0.0.1:1020"}}
            ]
        }"#;
        let tmp_dir = tempfile::tempdir().unwrap();
        let node_dir = tmp_dir.path().join("n");
        std::fs::create_dir(&node_dir).unwrap();
        let tmp_file = node_dir.join("setup.json");
        std::fs::write(&tmp_file, v1_json_json).unwrap();

        // Run migration
        let nodes_state = NodesState::new(tmp_dir.path());
        nodes_state.migrate(&node_dir).await.unwrap();

        // Check migration was done correctly
        let contents = std::fs::read_to_string(&tmp_file).unwrap();
        let v2_setup: NodeSetupConfig = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            v2_setup.api_transport,
            Some(CreateTransportJson {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                addr: InternetAddress::V4("127.0.0.1:1020".parse().unwrap())
            })
        );
    }
}
