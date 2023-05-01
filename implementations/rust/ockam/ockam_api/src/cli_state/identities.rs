use super::Result;
use crate::cli_state::nodes::NodeState;
use crate::cli_state::traits::{StateDirTrait, StateItemTrait};
use crate::cli_state::CliStateError;
use ockam_identity::{
    Identities, IdentitiesRepository, IdentitiesStorage, IdentitiesVault, Identity,
    IdentityHistoryComparison, IdentityIdentifier, LmdbStorage,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentitiesState {
    dir: PathBuf,
}

impl IdentitiesState {
    pub fn get_or_default(&self, name: Option<String>) -> Result<IdentityState> {
        if let Some(identity_name) = name {
            self.get(identity_name.as_ref())
        } else {
            self.default()
        }
    }

    pub fn get_by_identifier(&self, identifier: &IdentityIdentifier) -> Result<IdentityState> {
        let identities = self.list()?;

        let identity_state = identities
            .into_iter()
            .find(|ident_state| &ident_state.config.identity.identifier() == identifier);

        match identity_state {
            Some(is) => Ok(is),
            None => Err(CliStateError::NotFound),
        }
    }

    pub async fn identities_repository(&self) -> Result<Arc<dyn IdentitiesRepository>> {
        let lmdb_path = self.identities_repository_path()?;
        Ok(Arc::new(IdentitiesStorage::new(Arc::new(
            LmdbStorage::new(lmdb_path).await?,
        ))))
    }

    pub fn identities_repository_path(&self) -> Result<PathBuf> {
        let lmdb_path = self.dir.join("data").join("authenticated_storage.lmdb");
        Ok(lmdb_path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentityState {
    name: String,
    path: PathBuf,
    /// The path to the directory containing the authenticated storage files, shared amongst all identities
    data_path: PathBuf,
    config: IdentityConfig,
}

impl IdentityState {
    pub async fn get(&self, vault: Arc<dyn IdentitiesVault>) -> Result<Identity> {
        let data = self.config.identity.export()?;
        Ok(self
            .make_identities(vault)
            .await?
            .identities_creation()
            .import_identity(&data)
            .await?)
    }

    pub async fn make_identities(
        &self,
        vault: Arc<dyn IdentitiesVault>,
    ) -> Result<Arc<Identities>> {
        let repository = self.cli_state()?.identities.identities_repository().await?;
        Ok(Identities::builder()
            .with_identities_vault(vault)
            .with_identities_repository(repository)
            .build())
    }

    pub fn set_enrollment_status(&mut self) -> Result<()> {
        self.config.enrollment_status = Some(EnrollmentStatus::enrolled());
        self.persist()
    }

    fn build_data_path(path: &Path) -> PathBuf {
        path.parent().expect("Should have parent").join("data")
    }

    fn in_use(&self) -> Result<()> {
        self.in_use_by(&self.cli_state()?.nodes.list()?)
    }

    fn in_use_by(&self, nodes: &[NodeState]) -> Result<()> {
        for node in nodes {
            if node.config().identity_config()?.identity.identifier()
                == self.config.identity.identifier()
            {
                return Err(CliStateError::Invalid(format!(
                    "Can't delete identity '{}' because is currently in use by node '{}'",
                    &self.name,
                    &node.name()
                )));
            }
        }
        Ok(())
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
        writeln!(
            f,
            "Config Identifier: {}",
            self.config.identity.identifier()
        )?;
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
    pub identity: Identity,
    pub enrollment_status: Option<EnrollmentStatus>,
}

impl IdentityConfig {
    pub async fn new(identity: &Identity) -> Self {
        Self {
            identity: identity.clone(),
            enrollment_status: None,
        }
    }

    pub fn identity(&self) -> Identity {
        self.identity.clone()
    }
}

impl PartialEq for IdentityConfig {
    fn eq(&self, other: &Self) -> bool {
        self.identity.compare(&other.identity) == IdentityHistoryComparison::Equal
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

        match OffsetDateTime::from(self.created_at).format(&Iso8601::DEFAULT) {
            Ok(time_str) => writeln!(f, "Timestamp: {}", time_str)?,
            Err(err) => writeln!(
                f,
                "Error formatting OffsetDateTime as Iso8601 String: {}",
                err
            )?,
        }

        Ok(())
    }
}

mod traits {
    use super::*;
    use crate::cli_state::traits::*;
    use crate::cli_state::{file_stem, CliStateError};
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateDirTrait for IdentitiesState {
        type Item = IdentityState;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn default_filename() -> &'static str {
            "identity"
        }

        fn build_dir(root_path: &Path) -> PathBuf {
            root_path.join("identities")
        }

        fn has_data_dir() -> bool {
            true
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn delete(&self, name: &str) -> Result<()> {
            // Retrieve identity. If doesn't exist do nothing.
            let identity = match self.get(name) {
                Ok(i) => i,
                Err(CliStateError::NotFound) => return Ok(()),
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
            identity.delete()?;
            Ok(())
        }
    }

    #[async_trait]
    impl StateItemTrait for IdentityState {
        type Config = IdentityConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            let name = file_stem(&path)?;
            let data_path = IdentityState::build_data_path(&path);
            Ok(Self {
                name,
                path,
                data_path,
                config,
            })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let name = file_stem(&path)?;
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            let data_path = IdentityState::build_data_path(&path);
            Ok(Self {
                name,
                path,
                data_path,
                config,
            })
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn data_path(&self) -> Option<&PathBuf> {
            Some(&self.data_path)
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
