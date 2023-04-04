use crate::cloud::project::Project;

use crate::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};

use nix::errno::Errno;
use ockam_core::compat::sync::Arc;
use ockam_identity::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, PublicIdentity, SecureChannelRegistry,
};
use ockam_vault::{storage::FileStorage, Vault};
use rand::random;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use sysinfo::{Pid, System, SystemExt};

use crate::lmdb::LmdbStorage;
use ockam_core::env::get_env_with_default;
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::credential::Credential;
use thiserror::Error;

type Result<T> = std::result::Result<T, CliStateError>;

#[derive(Debug, Error)]
pub enum CliStateError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
    #[error("ockam error")]
    Ockam(#[from] ockam_core::Error),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("not found: {0}")]
    NotFound(String),
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
            vaults: VaultsState::new(dir)?,
            identities: IdentitiesState::new(dir)?,
            nodes: NodesState::new(dir)?,
            projects: ProjectsState::new(dir)?,
            credentials: CredentialsState::new(dir)?,
            dir: dir.to_path_buf(),
        })
    }

    pub fn test() -> Result<Self> {
        let tests_dir = dirs::home_dir()
            .ok_or_else(|| CliStateError::NotFound("home dir".to_string()))?
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
            &self.identities.dir,
            &self.vaults.dir,
            &self.projects.dir,
            &self.credentials.dir,
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
                .ok_or_else(|| CliStateError::NotFound("home dir".to_string()))?
                .join(".ockam"),
        )?)
    }

    /// Returns the directory where the default objects are stored.
    fn defaults_dir(dir: &Path) -> Result<PathBuf> {
        Ok(dir.join("defaults"))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultsState {
    dir: PathBuf,
}

impl VaultsState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("vaults");
        std::fs::create_dir_all(dir.join("data"))?;
        Ok(Self { dir })
    }

    pub fn directory(&self) -> String {
        self.dir.to_string_lossy().into()
    }

    pub async fn create(&self, name: &str, config: VaultConfig) -> Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if path.exists() {
                return Err(CliStateError::AlreadyExists(format!("vault `{name}`")));
            }
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        let state = VaultState {
            name: name.to_string(),
            path,
            config,
        };
        state.get().await?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }

    pub fn get(&self, name: &str) -> Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("vault `{name}`")));
            }
            path
        };
        VaultState::try_from(&path)
    }

    pub fn list(&self) -> Result<Vec<VaultState>> {
        let mut vaults = Vec::default();
        for entry in std::fs::read_dir(&self.dir)? {
            if let Ok(vault) = self.get(&file_stem(&entry?.path())?) {
                vaults.push(vault);
            }
        }
        Ok(vaults)
    }

    pub async fn delete(&self, name: &str) -> Result<()> {
        // Retrieve vault. If doesn't exist do nothing.
        let vault = match self.get(name) {
            Ok(v) => v,
            Err(CliStateError::NotFound(_)) => return Ok(()),
            Err(e) => return Err(e),
        };

        // If it's the default, remove link
        if let Ok(default) = self.default() {
            if default.path == vault.path {
                let _ = std::fs::remove_file(self.default_path()?);
            }
        }

        // Remove vault files
        vault.delete()?;

        Ok(())
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(CliState::defaults_dir(self.dir.parent().expect("Should have parent"))?.join("vault"))
    }

    pub fn default(&self) -> Result<VaultState> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        VaultState::try_from(&path)
    }

    pub fn set_default(&self, name: &str) -> Result<VaultState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("vault `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        // Remove link if it exists
        let _ = std::fs::remove_file(&link);
        // Create link to the identity
        std::os::unix::fs::symlink(original, link)?;
        self.get(name)
    }

    pub fn is_default(&self, name: &str) -> Result<bool> {
        let _exists = self.get(name)?;
        let default_name = {
            let path = std::fs::canonicalize(self.default_path()?)?;
            file_stem(&path)?
        };
        Ok(default_name.eq(name))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultState {
    pub name: String,
    pub path: PathBuf,
    pub config: VaultConfig,
}

impl VaultState {
    pub fn name(&self) -> Result<String> {
        self.path
            .file_stem()
            .and_then(|s| s.to_os_string().into_string().ok())
            .ok_or_else(|| {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "failed to parse the vault name",
                ))
            })
    }

    fn data_path(&self, name: &str) -> Result<PathBuf> {
        Ok(self
            .path
            .parent()
            .expect("Should have parent")
            .join("data")
            .join(format!("{name}-storage.json")))
    }

    pub async fn get(&self) -> Result<Vault> {
        let vault_storage = FileStorage::create(self.vault_file_path()?).await?;
        let mut vault = Vault::new(Some(Arc::new(vault_storage)));
        if self.config.aws_kms {
            vault.enable_aws_kms().await?
        }
        Ok(vault)
    }

    pub fn vault_file_path(&self) -> Result<PathBuf> {
        self.data_path(&self.name)
    }

    pub fn delete(&self) -> Result<()> {
        std::fs::remove_file(&self.path)?;
        let data_path = self.data_path(&self.name)?;
        std::fs::remove_file(&data_path)?;
        std::fs::remove_file(data_path.with_extension("json.lock"))?;
        Ok(())
    }
}

impl Display for VaultState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Name: {}",
            self.path.as_path().file_stem().unwrap().to_str().unwrap()
        )?;
        writeln!(
            f,
            "Type: {}",
            match self.config.is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
        )?;
        Ok(())
    }
}

impl TryFrom<&PathBuf> for VaultState {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let name = file_stem(path)?;
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(Self {
            name,
            path: path.clone(),
            config,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct VaultConfig {
    #[serde(default)]
    aws_kms: bool,
}

impl VaultConfig {
    pub fn new(aws_kms: bool) -> Result<Self> {
        Ok(Self { aws_kms })
    }

    pub fn is_aws(&self) -> bool {
        self.aws_kms
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentitiesState {
    dir: PathBuf,
}

impl IdentitiesState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("identities");
        std::fs::create_dir_all(dir.join("data"))?;
        Ok(Self { dir })
    }

    pub fn directory(&self) -> String {
        self.dir.to_string_lossy().into()
    }

    pub fn create(&self, name: &str, config: IdentityConfig) -> Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if path.exists() {
                return Err(CliStateError::AlreadyExists(format!("identity `{name}`")));
            }
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        let state = IdentityState {
            name: name.to_string(),
            path,
            config,
        };
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(state)
    }

    pub fn get(&self, name: &str) -> Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("identity `{name}`")));
            }
            path
        };
        IdentityState::try_from(&path)
    }

    pub fn get_by_identifier(&self, identifier: &IdentityIdentifier) -> Result<IdentityState> {
        let identities = self.list()?;

        let identity_state = identities
            .into_iter()
            .find(|ident_state| &ident_state.config.identifier == identifier);

        match identity_state {
            Some(is) => Ok(is),
            None => Err(CliStateError::NotFound(format!(
                "identity with identifier `{identifier}`"
            ))),
        }
    }

    pub fn list(&self) -> Result<Vec<IdentityState>> {
        let mut identities: Vec<IdentityState> = vec![];
        for entry in std::fs::read_dir(&self.dir)? {
            if let Ok(identity) = self.get(&file_stem(&entry?.path())?) {
                identities.push(identity);
            }
        }
        Ok(identities)
    }

    pub async fn delete(&self, name: &str) -> Result<()> {
        // Retrieve identity. If doesn't exist do nothing.
        let identity = match self.get(name) {
            Ok(i) => i,
            Err(CliStateError::NotFound(_)) => return Ok(()),
            Err(e) => return Err(e),
        };

        // Abort if identity is being used by some running node.
        identity.in_use()?;

        // If it's the default, remove link
        if let Ok(default) = self.default() {
            if default.path == identity.path {
                let _ = std::fs::remove_file(self.default_path()?);
            }
        }

        // Remove identity file
        std::fs::remove_file(identity.path)?;

        Ok(())
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(
            CliState::defaults_dir(self.dir.parent().expect("Should have parent"))?
                .join("identity"),
        )
    }

    pub fn default(&self) -> Result<IdentityState> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        IdentityState::try_from(&path)
    }

    pub fn is_default(&self, name: &str) -> Result<bool> {
        let _exists = self.get(name)?;
        let default_name = {
            let path = std::fs::canonicalize(self.default_path()?)?;
            file_stem(&path)?
        };
        Ok(default_name.eq(name))
    }

    pub fn set_default(&self, name: &str) -> Result<IdentityState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("identity `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        // Remove link if it exists
        let _ = std::fs::remove_file(&link);
        // Create link to the identity
        std::os::unix::fs::symlink(original, &link)?;
        self.get(name)
    }

    pub async fn authenticated_storage(&self) -> Result<Arc<dyn AuthenticatedStorage>> {
        let lmdb_path = self.authenticated_storage_path()?;
        Ok(Arc::new(LmdbStorage::new(lmdb_path).await?))
    }

    pub fn authenticated_storage_path(&self) -> Result<PathBuf> {
        let lmdb_path = self.dir.join("data").join("authenticated_storage.lmdb");
        Ok(lmdb_path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentityState {
    pub name: String,
    pub path: PathBuf,
    pub config: IdentityConfig,
}

impl IdentityState {
    fn persist(&self) -> Result<()> {
        let contents = serde_json::to_string(&self.config)?;
        std::fs::write(&self.path, contents)?;
        Ok(())
    }

    fn in_use(&self) -> Result<()> {
        let cli_state_path = self
            .path
            .parent()
            .expect("Should have identities dir as parent")
            .parent()
            .expect("Should have CliState dir as parent");
        self.in_use_by(&CliState::new(cli_state_path)?.nodes.list()?)
    }

    fn in_use_by(&self, nodes: &[NodeState]) -> Result<()> {
        for node in nodes {
            if node.config.identity_config()?.identifier == self.config.identifier {
                return Err(CliStateError::Invalid(format!(
                    "Can't delete identity '{}' because is currently in use by node '{}'",
                    &self.name, &node.config.name
                )));
            }
        }
        Ok(())
    }

    pub fn set_enrollment_status(&mut self) -> Result<()> {
        self.config.enrollment_status = Some(EnrollmentStatus::enrolled());
        self.persist()
    }

    pub async fn get(
        &self,
        ctx: &ockam::Context,
        vault: Arc<dyn IdentityVault>,
    ) -> Result<Identity> {
        let data = self.config.change_history.export()?;
        let cli_state_path = self
            .path
            .parent()
            .expect("Should have identities dir as parent")
            .parent()
            .expect("Should have CliState dir as parent");
        let storage = CliState::new(cli_state_path)?
            .identities
            .authenticated_storage()
            .await?;
        Ok(Identity::import_ext(
            ctx,
            &data,
            storage,
            &SecureChannelRegistry::new(),
            vault.clone(),
        )
        .await?)
    }
}

impl TryFrom<&PathBuf> for IdentityState {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let name = file_stem(path)?;
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState {
            name,
            path: path.clone(),
            config,
        })
    }
}

impl Display for IdentityState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Name: {}",
            self.path.as_path().file_stem().unwrap().to_str().unwrap()
        )?;
        writeln!(f, "State Path: {}", self.path.clone().to_str().unwrap())?;
        writeln!(f, "Config Identifier: {}", self.config.identifier)?;
        match &self.config.enrollment_status {
            Some(enrollment) => {
                writeln!(f, "Enrollment Status:")?;
                for line in enrollment.to_string().lines() {
                    writeln!(f, "{:2}{}", "", line)?;
                }
            }
            None => (),
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityConfig {
    pub identifier: IdentityIdentifier,
    pub change_history: IdentityChangeHistory,
    pub enrollment_status: Option<EnrollmentStatus>,
}

impl IdentityConfig {
    pub async fn new(identity: &Identity) -> Self {
        let identifier = identity.identifier().clone();
        let change_history = identity.change_history().await;
        Self {
            identifier,
            change_history,
            enrollment_status: None,
        }
    }

    pub fn public_identity(&self) -> PublicIdentity {
        PublicIdentity::new(self.identifier.clone(), self.change_history.clone())
    }
}

impl PartialEq for IdentityConfig {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
            && self.change_history.compare(&other.change_history)
                == IdentityHistoryComparison::Equal
    }
}

impl Eq for IdentityConfig {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrollmentStatus {
    pub is_enrolled: bool,
    pub created_at: SystemTime,
}

impl EnrollmentStatus {
    pub fn enrolled() -> EnrollmentStatus {
        EnrollmentStatus {
            is_enrolled: true,
            created_at: SystemTime::now(),
        }
    }
}

impl Display for EnrollmentStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_enrolled {
            writeln!(f, "Enrolled: yes")?;
        } else {
            writeln!(f, "Enrolled: no")?;
        }

        // FIX: it fails to compile in some environments
        // match OffsetDateTime::from(self.created_at).format(&Iso8601::DEFAULT) {
        //     Ok(time_str) => writeln!(f, "Timestamp: {}", time_str)?,
        //     Err(err) => writeln!(
        //         f,
        //         "Error formatting OffsetDateTime as Iso8601 String: {}",
        //         err
        //     )?,
        // }

        Ok(())
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
                return Err(CliStateError::NotFound(format!("node `{name}`")));
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

    pub fn create(&self, name: &str, mut config: NodeConfig) -> Result<NodeState> {
        config.name = name.to_string();

        // check if node name already exists
        if self.get_node_path(name).exists() {
            return Err(CliStateError::AlreadyExists(format!("node `{name}`")));
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
            Err(CliStateError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn get(&self, name: &str) -> Result<NodeState> {
        let path = self.get_node_path(name);
        if !path.exists() {
            return Err(CliStateError::NotFound(format!("node `{name}`")));
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
            Err(CliStateError::NotFound(_)) => return Ok(()),
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
            default_vault: cli_state.vaults.default()?.path,
            default_identity: cli_state.identities.default()?.path,
            setup: NodeSetupConfig::default(),
        })
    }

    pub async fn vault(&self) -> Result<Vault> {
        let state_path = std::fs::canonicalize(&self.default_vault)?;
        let state = VaultState::try_from(&state_path)?;
        state.get().await
    }

    pub fn identity_config(&self) -> Result<IdentityConfig> {
        let path = std::fs::canonicalize(&self.default_identity)?;
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub async fn identity(&self, ctx: &ockam::Context) -> Result<Identity> {
        let vault: Arc<dyn IdentityVault> = Arc::new(self.vault().await?);
        let state_path = std::fs::canonicalize(&self.default_identity)?;
        let state = IdentityState::try_from(&state_path)?;
        state.get(ctx, vault).await
    }
}

impl TryFrom<&CliState> for NodeConfig {
    type Error = CliStateError;

    fn try_from(cli_state: &CliState) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: random_name(),
            version: NodeConfigVersion::latest(),
            default_vault: cli_state.vaults.default()?.path,
            default_identity: cli_state.identities.default()?.path,
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
            None => cli_state.vaults.default()?.path,
        };
        let identity = match self.identity {
            Some(path) => path,
            None => cli_state.identities.default()?.path,
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

    pub fn default_tcp_listener(&self) -> Result<&CreateTransportJson> {
        self.transports
            .iter()
            .find(|t| t.tt == TransportType::Tcp && t.tm == TransportMode::Listen)
            .ok_or_else(|| CliStateError::NotFound("default tcp transport".to_string()))
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectsState {
    dir: PathBuf,
}

impl ProjectsState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("projects");
        std::fs::create_dir_all(dir.join("data"))?;
        Ok(Self { dir })
    }

    pub fn create(&self, name: &str, config: Project<'_>) -> Result<ProjectState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }

        Ok(ProjectState { path })
    }

    pub fn get(&self, name: &str) -> Result<ProjectState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("project `{name}`")));
            }
            path
        };
        Ok(ProjectState { path })
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(CliState::defaults_dir(self.dir.parent().expect("Should have parent"))?.join("project"))
    }

    pub fn default(&self) -> Result<ProjectState> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        Ok(ProjectState { path })
    }

    pub fn set_default(&self, name: &str) -> Result<ProjectState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("project `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        std::os::unix::fs::symlink(original, link)?;
        self.get(name)
    }

    pub fn list(&self) -> Result<Vec<ProjectState>> {
        let mut projects = vec![];
        for entry in std::fs::read_dir(&self.dir)? {
            if let Ok(project) = self.get(&file_stem(&entry?.path())?) {
                projects.push(project);
            }
        }
        Ok(projects)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        // Retrieve project. If doesn't exist do nothing.
        let project = match self.get(name) {
            Ok(project) => project,
            Err(CliStateError::NotFound(_)) => return Ok(()),
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectState {
    pub path: PathBuf,
}

impl ProjectState {
    pub fn name(&self) -> Result<String> {
        self.path
            .file_stem()
            .and_then(|s| s.to_os_string().into_string().ok())
            .ok_or_else(|| {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "failed to parse the project name",
                ))
            })
    }

    fn delete(&self) -> Result<()> {
        let _ = std::fs::remove_file(&self.path);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialConfig {
    pub issuer: PublicIdentity,
    pub encoded_credential: String,
}

impl CredentialConfig {
    pub fn new(issuer: PublicIdentity, encoded_credential: String) -> Result<Self> {
        Ok(Self {
            issuer,
            encoded_credential,
        })
    }

    pub fn credential(&self) -> Result<Credential> {
        let bytes = match hex::decode(&self.encoded_credential) {
            Ok(b) => b,
            Err(e) => {
                return Err(CliStateError::Invalid(format!(
                    "Unable to hex decode credential. {e}"
                )))
            }
        };
        minicbor::decode::<Credential>(&bytes)
            .map_err(|e| CliStateError::Invalid(format!("Unable to decode credential. {e}")))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CredentialsState {
    dir: PathBuf,
}

impl CredentialsState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("credentials");
        std::fs::create_dir_all(dir.join("data"))?;
        Ok(Self { dir })
    }

    pub async fn create(&self, name: &str, cred: CredentialConfig) -> Result<CredentialState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if path.exists() {
                return Err(CliStateError::AlreadyExists(format!("credential `{name}`")));
            }
            path
        };
        let contents = serde_json::to_string(&cred)?;
        std::fs::write(&path, contents)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }

        Ok(CredentialState { path })
    }

    pub fn get(&self, name: &str) -> Result<CredentialState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("credential `{name}`")));
            }
            path
        };
        Ok(CredentialState { path })
    }

    pub fn list(&self) -> Result<Vec<CredentialState>> {
        let mut creds = vec![];
        for entry in std::fs::read_dir(&self.dir)? {
            if let Ok(cred) = self.get(&file_stem(&entry?.path())?) {
                creds.push(cred);
            }
        }

        Ok(creds)
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(
            CliState::defaults_dir(self.dir.parent().expect("Should have parent"))?
                .join("credential"),
        )
    }

    pub fn default(&self) -> Result<CredentialState> {
        let path = std::fs::canonicalize(self.default_path()?)?;
        Ok(CredentialState { path })
    }

    pub fn set_default(&self, name: &str) -> Result<CredentialState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{name}.json"));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("credential `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        std::os::unix::fs::symlink(original, link)?;
        self.get(name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct CredentialState {
    pub path: PathBuf,
}

impl CredentialState {
    pub fn config(&self) -> Result<CredentialConfig> {
        let string_config = std::fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&string_config)?)
    }

    pub fn name(&self) -> Result<String> {
        self.path
            .file_stem()
            .and_then(|s| s.to_os_string().into_string().ok())
            .ok_or_else(|| {
                CliStateError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "failed to parse the credentials name",
                ))
            })
    }
}

pub fn random_name() -> String {
    hex::encode(random::<[u8; 4]>())
}

fn file_stem(path: &Path) -> Result<String> {
    path.file_stem()
        .ok_or_else(|| CliStateError::NotFound(format!("name for {path:?}")))?
        .to_str()
        .map(|name| name.to_string())
        .ok_or_else(|| CliStateError::NotFound(format!("name for {path:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // This tests way too many different things
    #[ockam_macros::test(crate = "ockam")]
    async fn integration(ctx: &mut ockam::Context) -> ockam::Result<()> {
        let sut = CliState::test()?;

        // Vaults
        let vault_name = {
            let name = hex::encode(random::<[u8; 4]>());
            let config = VaultConfig::default();

            let state = sut.vaults.create(&name, config).await.unwrap();
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
            let vault: Arc<dyn IdentityVault> = Arc::new(vault_state.get().await.unwrap());
            let identity =
                Identity::create_ext(ctx, sut.identities.authenticated_storage().await?, vault)
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
            let config = Project::default();

            let state = sut.projects.create(&name, config).unwrap();
            let got = sut.projects.get(&name).unwrap();
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
            "projects/data".to_string(),
            format!("projects/{project_name}.json"),
            "credentials".to_string(),
            "credentials/data".to_string(),
            "defaults".to_string(),
            "defaults/vault".to_string(),
            "defaults/identity".to_string(),
            "defaults/node".to_string(),
            "defaults/project".to_string(),
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
                        let entry_name = entry.file_name().into_string().unwrap();
                        if entry.path().is_dir() {
                            assert_eq!(entry_name, "data");
                            found_entries.push(format!("{dir_name}/{entry_name}"));
                        } else {
                            assert!(entry.path().is_file());
                            let file_name = entry.file_name().into_string().unwrap();
                            found_entries.push(format!("{dir_name}/{file_name}"));
                        }
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
                _ => panic!("unexpected file"),
            }
        });
        found_entries.sort();
        assert_eq!(expected_entries, found_entries);

        sut.projects.delete(&project_name).unwrap();
        sut.nodes.delete(&node_name, false).unwrap();
        sut.identities.delete(&identity_name).await.unwrap();
        sut.vaults.delete(&vault_name).await.unwrap();

        ctx.stop().await?;
        Ok(())
    }
}
