pub mod credentials;
pub mod identities;
pub mod nodes;
pub mod projects;
pub mod traits;
pub mod trust_contexts;
pub mod vaults;

pub use crate::cli_state::credentials::*;
pub use crate::cli_state::identities::*;
pub use crate::cli_state::nodes::*;
pub use crate::cli_state::projects::*;
pub use crate::cli_state::traits::*;
pub use crate::cli_state::trust_contexts::*;
pub use crate::cli_state::vaults::*;
use ockam::identity::Identities;
use ockam_core::compat::sync::Arc;
use ockam_core::env::get_env_with_default;
use ockam_vault::Vault;
use rand::random;
use std::path::{Path, PathBuf};
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
            nodes: NodesState::load(dir)?,
            projects: ProjectsState::load(dir)?,
            credentials: CredentialsState::load(dir)?,
            trust_contexts: TrustContextsState::load(dir)?,
            dir: dir.to_path_buf(),
        })
    }

    pub fn test() -> Result<Self> {
        let tests_dir = home::home_dir()
            .ok_or(CliStateError::NotFound)?
            .join(".ockam")
            .join(".tests")
            .join(random_name());
        Self::new(&tests_dir)
    }

    pub fn delete(&self, force: bool) -> Result<()> {
        // Delete all nodes
        for n in self.nodes.list()? {
            let _ = n.delete_sigkill(force);
        }

        let dir = &self.dir;
        for dir in &[
            (self.nodes.dir()),
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
            home::home_dir()
                .ok_or(CliStateError::NotFound)?
                .join(".ockam"),
        )?)
    }

    /// Returns the directory where the default objects are stored.
    fn defaults_dir(dir: &Path) -> Result<PathBuf> {
        Ok(dir.join("defaults"))
    }

    pub async fn create_vault_state(&self, vault_name: Option<&str>) -> Result<VaultState> {
        let vault_state = if let Some(v) = vault_name {
            self.vaults.get(v)?
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
        identity_name: Option<&str>,
        vault: Arc<Vault>,
    ) -> Result<IdentityState> {
        if let Ok(identity) = self.identities.get_or_default(identity_name) {
            Ok(identity)
        } else {
            self.make_identity_state(vault, identity_name).await
        }
    }

    async fn make_identity_state(
        &self,
        vault: Arc<Vault>,
        name: Option<&str>,
    ) -> Result<IdentityState> {
        let identity = self
            .get_identities(vault)
            .await?
            .identities_creation()
            .create_identity()
            .await?;
        let identity_config = IdentityConfig::new(&identity).await;
        let identity_name = name
            .map(|x| x.to_string())
            .unwrap_or_else(|| hex::encode(random::<[u8; 4]>()));
        self.identities.create(&identity_name, identity_config)
    }

    pub async fn get_identities(&self, vault: Arc<Vault>) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .with_identities_vault(vault)
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
    use ockam_identity::IdentitiesVault;

    #[tokio::test]
    async fn test_create_default_identity_state() {
        let state = CliState::test().unwrap();
        let vault = Vault::create();
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
        let vault = Vault::create();
        let identity1 = state
            .create_identity_state(Some("alice"), vault.clone())
            .await
            .unwrap();
        let identity2 = state
            .create_identity_state(Some("alice"), vault)
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
            let vault: Arc<dyn IdentitiesVault> = vault_state.get().await.unwrap();
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
        sut.nodes.delete(&node_name).unwrap();
        sut.identities.delete(&identity_name).unwrap();
        sut.vaults.delete(&vault_name).unwrap();

        ctx.stop().await?;
        Ok(())
    }
}
