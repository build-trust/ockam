// use crate::util::startup;
use ockam_identity::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use ockam_identity::{Identity, IdentityIdentifier};
use ockam_vault::{storage::FileStorage, Vault};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
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
    #[error("`{0}` already exists")]
    AlreadyExists(String),
    #[error("`{0}` not found")]
    NotFound(String),
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

#[derive(Clone)]
pub struct CliState {
    pub vaults: VaultsState,
    pub identities: IdentitiesState,
    pub nodes: NodesState,
    dir: PathBuf,
}

impl CliState {
    pub fn new() -> Result<Self> {
        let dir = Self::dir()?;
        std::fs::create_dir_all(dir.join("defaults"))?;
        Ok(Self {
            vaults: VaultsState::new(&dir)?,
            identities: IdentitiesState::new(&dir)?,
            nodes: NodesState::new(&dir)?,
            dir,
        })
    }

    pub fn dir() -> Result<PathBuf> {
        Ok(match std::env::var("OCKAM_HOME") {
            Ok(dir) => PathBuf::from(&dir),
            Err(_) => dirs::home_dir()
                .ok_or_else(|| CliStateError::NotFound("home dir".to_string()))?
                .join(".ockam"),
        })
    }

    fn defaults_dir() -> Result<PathBuf> {
        Ok(Self::dir()?.join("defaults"))
    }
}

#[derive(Clone)]
pub struct VaultsState {
    dir: PathBuf,
}

impl VaultsState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("vaults");
        std::fs::create_dir_all(dir.join("data"))?;
        Ok(Self { dir })
    }

    pub async fn create(&self, name: &str, config: VaultConfig) -> Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if path.exists() {
                return Err(CliStateError::AlreadyExists(format!("vault `{name}`")));
            }
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        config.get().await?;
        Ok(VaultState { path, config })
    }

    pub fn get(&self, name: &str) -> Result<VaultState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("vault `{name}`")));
            }
            path
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(VaultState { path, config })
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(CliState::defaults_dir()?.join("vault"))
    }

    pub fn default(&self) -> Result<VaultState> {
        let path = std::fs::canonicalize(&self.default_path()?)?;
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(VaultState { path, config })
    }

    pub fn set_default(&self, name: &str) -> Result<VaultState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("vault `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        std::os::unix::fs::symlink(&original, &link)?;
        let contents = std::fs::read_to_string(&original)?;
        Ok(VaultState {
            path: original,
            config: serde_json::from_str(&contents)?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VaultState {
    path: PathBuf,
    pub config: VaultConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum VaultConfig {
    Fs { path: PathBuf },
}

impl VaultConfig {
    pub async fn get(&self) -> Result<Vault> {
        match &self {
            VaultConfig::Fs { path } => {
                let vault_storage = FileStorage::create(path.clone()).await?;
                let vault = Vault::new(Some(Arc::new(vault_storage)));
                Ok(vault)
            }
        }
    }

    pub fn fs_path(name: &str, path: impl Into<Option<String>>) -> Result<PathBuf> {
        Ok(if let Some(path) = path.into() {
            PathBuf::from(path)
        } else {
            let state = CliState::new()?;
            state
                .vaults
                .dir
                .join("data")
                .join(format!("{name}-storage.json"))
        })
    }

    pub fn fs(path: PathBuf) -> Result<Self> {
        Ok(Self::Fs { path })
    }

    pub fn fs_default(name: &str) -> Result<Self> {
        Ok(Self::Fs {
            path: Self::fs_path(name, None)?,
        })
    }
}

#[derive(Clone)]
pub struct IdentitiesState {
    dir: PathBuf,
}

impl IdentitiesState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("identities");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn create(&self, name: &str, config: IdentityConfig) -> Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if path.exists() {
                return Err(CliStateError::AlreadyExists(format!("identity `{name}`")));
            }
            path
        };
        let contents = serde_json::to_string(&config)?;
        std::fs::write(&path, contents)?;
        if !self.default_path()?.exists() {
            self.set_default(name)?;
        }
        Ok(IdentityState { path, config })
    }

    pub fn get(&self, name: &str) -> Result<IdentityState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("identity `{name}`")));
            }
            path
        };
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState { path, config })
    }

    pub fn default_path(&self) -> Result<PathBuf> {
        Ok(CliState::defaults_dir()?.join("identity"))
    }

    pub fn default(&self) -> Result<IdentityState> {
        let path = std::fs::canonicalize(&self.default_path()?)?;
        let contents = std::fs::read_to_string(&path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState { path, config })
    }

    pub fn set_default(&self, name: &str) -> Result<IdentityState> {
        let original = {
            let mut path = self.dir.clone();
            path.push(format!("{}.json", name));
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("identity `{name}`")));
            }
            path
        };
        let link = self.default_path()?;
        std::os::unix::fs::symlink(&original, &link)?;
        let contents = std::fs::read_to_string(&original)?;
        let config = serde_json::from_str(&contents)?;
        Ok(IdentityState {
            path: original,
            config,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentityState {
    path: PathBuf,
    pub config: IdentityConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityConfig {
    pub identifier: IdentityIdentifier,
    pub change_history: IdentityChangeHistory,
}

impl IdentityConfig {
    pub async fn new(identity: &Identity<Vault>) -> Self {
        let identifier = identity.identifier().clone();
        let change_history = identity.change_history().await;
        Self {
            identifier,
            change_history,
        }
    }

    pub async fn get(&self, ctx: &ockam::Context, vault: &Vault) -> Result<Identity<Vault>> {
        let data = self.change_history.export()?;
        Ok(Identity::import(ctx, &data, vault).await?)
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

#[derive(Clone)]
pub struct NodesState {
    pub dir: PathBuf,
}

impl NodesState {
    fn new(cli_path: &Path) -> Result<Self> {
        let dir = cli_path.join("nodes");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn create(&self, name: &str, config: NodeConfig) -> Result<NodeState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            std::fs::create_dir_all(&path)?;
            path
        };
        let state = NodeState::new(path, config);
        std::fs::write(state.path.join("version"), state.config.version.to_string())?;
        if let Some(pid) = state.config.pid {
            std::fs::write(state.path.join("pid"), pid.to_string())?;
        }
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
        Ok(state)
    }

    pub fn get(&self, name: &str) -> Result<NodeState> {
        let path = {
            let mut path = self.dir.clone();
            path.push(name);
            if !path.exists() {
                return Err(CliStateError::NotFound(format!("node `{name}`")));
            }
            path
        };
        let config = NodeConfig::try_from(&path)?;
        Ok(NodeState::new(path, config))
    }

    fn vault_name(&self, name: &str) -> Result<String> {
        let mut path = self.dir.clone();
        path.push(name);
        path.push("default_vault");
        let path = std::fs::canonicalize(&path)?;
        file_stem(&path)
    }

    fn identity_name(&self, name: &str) -> Result<String> {
        let mut path = self.dir.clone();
        path.push(name);
        path.push("default_identity");
        let path = std::fs::canonicalize(&path)?;
        file_stem(&path)
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

    pub fn name(&self) -> &str {
        self.path.file_name().unwrap().to_str().unwrap()
    }

    pub fn socket(&self) -> PathBuf {
        self.path.join("socket")
    }

    pub fn stdout_log(&self) -> PathBuf {
        self.path.join("stdout.log")
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.path.join("stderr.log")
    }

    pub fn authenticated_storage_path(&self) -> PathBuf {
        self.path.join("authenticated_storage.lmdb")
    }

    pub fn policies_storage_path(&self) -> PathBuf {
        self.path.join("policies_storage.lmdb")
    }

    pub fn delete(&self, sigkill: bool) -> Result<()> {
        // if let Some(pid) = self.config.pid {
        //     startup::stop(pid, sigkill)?;
        // }
        std::fs::remove_dir_all(&self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeConfig {
    version: NodeConfigVersion,
    pid: Option<i32>,
    default_vault: PathBuf,
    default_identity: PathBuf,
    // TODO
    // authorities: AuthoritiesConfig,
    // setup: NodeSetupConfig,
}

impl NodeConfig {
    pub fn default() -> Result<Self> {
        let cli_state = CliState::new()?;
        Ok(Self {
            version: NodeConfigVersion::latest(),
            pid: None,
            default_vault: cli_state.vaults.default()?.path,
            default_identity: cli_state.identities.default()?.path,
        })
    }

    pub async fn vault(&self) -> Result<Vault> {
        let path = std::fs::canonicalize(&self.default_vault)?;
        let config: VaultConfig = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        config.get().await
    }

    pub async fn identity(&self, ctx: &ockam::Context) -> Result<Identity<Vault>> {
        let vault = self.vault().await?;
        let path = std::fs::canonicalize(&self.default_identity)?;
        let config: IdentityConfig = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        config.get(ctx, &vault).await
    }
}

impl TryFrom<&PathBuf> for NodeConfig {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let version = NodeConfigVersion::load(path)?;
        let pid = {
            let path = path.join("pid");
            if path.exists() {
                Some(
                    std::fs::read_to_string(path.join("pid"))?
                        .parse::<i32>()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                )
            } else {
                None
            }
        };
        let default_vault = std::fs::canonicalize(path.join("default_vault"))?;
        let default_identity = std::fs::canonicalize(path.join("default_identity"))?;
        Ok(Self {
            version,
            pid,
            default_vault,
            default_identity,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeConfigBuilder {
    pid: Option<i32>,
    vault: Option<PathBuf>,
    identity: Option<PathBuf>,
}

impl NodeConfigBuilder {
    pub fn pid(mut self, pid: i32) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn vault(mut self, path: PathBuf) -> Self {
        self.vault = Some(path);
        self
    }

    pub fn identity(mut self, path: PathBuf) -> Self {
        self.identity = Some(path);
        self
    }

    pub fn build(self) -> Result<NodeConfig> {
        let cli_state = CliState::new()?;
        let vault = match self.vault {
            Some(path) => path,
            None => cli_state.vaults.default()?.path,
        };
        let identity = match self.identity {
            Some(path) => path,
            None => cli_state.identities.default()?.path,
        };
        Ok(NodeConfig {
            version: NodeConfigVersion::latest(),
            pid: self.pid,
            default_vault: vault,
            default_identity: identity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeConfigVersion {
    V1,
}

impl NodeConfigVersion {
    const FILE_NAME: &'static str = "version";

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

// TODO
// pub struct NodeSetupConfig {
//     verbose: u8,
//     // transports: ?,
//     // secure_channels: ?,
//     // inlets: ?,
//     // outlets: ?,
//     // services: ?,
//
// }

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
    use tempfile::{tempdir, Builder};

    // This test way too many different things
    #[ockam_macros::test(crate = "ockam")]
    async fn integration(ctx: &mut ockam::Context) -> ockam::Result<()> {
        let rnd_dir = Builder::new().prefix("ockam-").tempdir().unwrap();
        std::env::set_var("OCKAM_HOME", rnd_dir.path());
        let sut = CliState::new().unwrap();

        // Vaults
        let vault_name = {
            let name = hex::encode(rand::random::<[u8; 4]>());

            let path = rnd_dir
                .path()
                .join("vaults")
                .join("data")
                .join(&format!("{name}.json"));
            let vault_storage = FileStorage::create(path.clone()).await?;
            let vault = Vault::new(Some(Arc::new(vault_storage)));

            let config = VaultConfig::Fs { path };

            let state = sut.vaults.create(&name, config).await.unwrap();
            let got = sut.vaults.get(&name).unwrap();
            assert_eq!(got, state);

            let got = sut.vaults.default().unwrap();
            assert_eq!(got, state);

            name
        };

        // Identities
        let identity_name = {
            let name = hex::encode(rand::random::<[u8; 4]>());
            let vault_config = sut.vaults.get(&vault_name).unwrap().config;
            let vault = vault_config.get().await.unwrap();
            let identity = Identity::create(ctx, &vault).await.unwrap();
            let identifier =
                IdentityIdentifier::from_key_id(&hex::encode(rand::random::<[u8; 32]>()));
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
            let name = hex::encode(rand::random::<[u8; 4]>());
            let config = NodeConfig::default().unwrap();

            let state = sut.nodes.create(&name, config).unwrap();
            let got = sut.nodes.get(&name).unwrap();
            assert_eq!(got, state);

            name
        };

        // Check structure
        let mut expected_entries = vec![
            "vaults".to_string(),
            format!("vaults/{vault_name}.json"),
            "vaults/data".to_string(),
            format!("vaults/data/{vault_name}.json"),
            "identities".to_string(),
            format!("identities/{identity_name}.json"),
            "nodes".to_string(),
            format!("nodes/{node_name}"),
            "defaults".to_string(),
            "defaults/vault".to_string(),
            "defaults/identity".to_string(),
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
                                found_entries.push(format!("{dir_name}/{entry_name}/{file_name}"));
                                assert_eq!(file_name, format!("{vault_name}.json"));
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
                        assert!(entry.path().is_file());
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
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
                        let file_name = entry.file_name().into_string().unwrap();
                        found_entries.push(format!("{dir_name}/{file_name}"));
                    });
                }
                _ => panic!("unexpected file"),
            }
        });
        found_entries.sort();
        assert_eq!(expected_entries, found_entries);
        ctx.stop().await?;
        Ok(())
    }
}
