use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

use ockam::identity::storage::LmdbStorage;
use ockam::identity::{Identifier, IdentitiesRepository, IdentitiesStorage};

use crate::cli_state::traits::{StateDirTrait, StateItemTrait};
use crate::cli_state::{CliStateError, DATA_DIR_NAME};

use super::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdentitiesState {
    dir: PathBuf,
}

impl IdentitiesState {
    pub fn get_or_default(&self, name: Option<&str>) -> Result<IdentityState> {
        if let Some(identity_name) = name {
            self.get(identity_name)
        } else {
            self.default()
        }
    }

    pub fn get_by_identifier(&self, identifier: &Identifier) -> Result<IdentityState> {
        self.list()?
            .into_iter()
            .find(|ident_state| &ident_state.config.identifier() == identifier)
            .ok_or(CliStateError::ResourceNotFound {
                resource: Self::default_filename().to_string(),
                name: identifier.to_string(),
            })
    }

    pub async fn identities_repository(&self) -> Result<Arc<dyn IdentitiesRepository>> {
        let lmdb_path = self.identities_repository_path()?;
        Ok(Arc::new(IdentitiesStorage::new(Arc::new(
            LmdbStorage::new(lmdb_path).await?,
        ))))
    }

    pub fn identities_repository_path(&self) -> Result<PathBuf> {
        let lmdb_path = self
            .dir
            .join(DATA_DIR_NAME)
            .join("authenticated_storage.lmdb");
        Ok(lmdb_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityState {
    name: String,
    path: PathBuf,
    /// The path to the directory containing the authenticated storage files, shared amongst all identities
    data_path: PathBuf,
    config: IdentityConfig,
}

impl IdentityState {
    pub fn identifier(&self) -> Identifier {
        self.config.identifier()
    }

    pub fn set_enrollment_status(&mut self) -> Result<()> {
        self.config.enrollment_status = Some(EnrollmentStatus::enrolled());
        self.persist()
    }

    fn build_data_path(path: &Path) -> PathBuf {
        path.parent()
            .expect("Should have parent")
            .join(DATA_DIR_NAME)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_enrolled(&self) -> bool {
        self.config
            .enrollment_status
            .as_ref()
            .map(|s| s.is_enrolled)
            .unwrap_or(false)
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
        writeln!(f, "Config Identifier: {}", self.config.identifier())?;
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
    pub identifier: Identifier,
    pub enrollment_status: Option<EnrollmentStatus>,
}

impl PartialEq for IdentityConfig {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
    }
}

impl Eq for IdentityConfig {}

impl IdentityConfig {
    pub async fn new(identifier: &Identifier) -> Self {
        Self {
            identifier: identifier.clone(),
            enrollment_status: None,
        }
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }
}

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

// TODO: No longer supported: consider deleting
#[derive(Deserialize, Debug, Clone)]
struct IdentityConfigV1 {
    // Easiest way to fail deserialization
    _non_existent_field: bool,
}

// TODO: No longer supported: consider deleting
#[derive(Deserialize, Debug, Clone)]
struct IdentityConfigV2 {
    // Easiest way to fail deserialization
    _non_existent_field: bool,
}

// TODO: No longer supported: consider deleting
#[derive(Deserialize, Debug, Clone)]
struct IdentityConfigV3 {
    // Easiest way to fail deserialization
    _non_existent_field: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum IdentityConfigs {
    V1(IdentityConfigV1),
    V2(IdentityConfigV2),
    V3(IdentityConfigV3),
    V4(IdentityConfig),
}

mod traits {
    use ockam_core::async_trait;

    use crate::cli_state::traits::*;
    use crate::cli_state::{file_stem, CliStateError};

    use super::*;

    #[async_trait]
    impl StateDirTrait for IdentitiesState {
        type Item = IdentityState;
        const DEFAULT_FILENAME: &'static str = "identity";
        const DIR_NAME: &'static str = "identities";
        const HAS_DATA_DIR: bool = true;

        fn new(root_path: &Path) -> Self {
            Self {
                dir: Self::build_dir(root_path),
            }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }

        fn delete(&self, name: impl AsRef<str>) -> Result<()> {
            // Retrieve identity. If doesn't exist do nothing.
            let identity = match self.get(&name) {
                Ok(i) => i,
                Err(CliStateError::ResourceNotFound { .. }) => return Ok(()),
                Err(e) => return Err(e),
            };

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

        async fn migrate(&self, path: &Path) -> Result<()> {
            let contents = std::fs::read_to_string(path)?;

            // read the configuration and migrate to the most recent format if an old format is found
            // the most recent configuration only contains an identity identifier, so if we find an
            // old format we store the full identity in the shared identities repository before
            // writing the most recent configuration format
            match serde_json::from_str(&contents)? {
                IdentityConfigs::V1(_) | IdentityConfigs::V2(_) | IdentityConfigs::V3(_) => {
                    return Err(CliStateError::InvalidVersion(
                        "Migration not supported for old Identities".to_string(),
                    ))
                }
                IdentityConfigs::V4(_) => {}
            }
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

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let identity_config = create_identity_config();
        let expected = create_identity_config_json();
        assert_eq!(serde_json::to_string(&identity_config).unwrap(), expected)
    }

    #[test]
    fn test_deserialize() {
        let json = create_identity_config_json();
        let actual: IdentityConfig = serde_json::from_str(json.as_str()).unwrap();
        let expected = create_identity_config();
        assert_eq!(actual, expected)
    }

    fn create_identity_config() -> IdentityConfig {
        let identifier = Identifier::try_from("Ifa804b7fca12a19eed206ae180b5b576860ae651").unwrap();
        IdentityConfig {
            identifier,
            enrollment_status: Some(EnrollmentStatus {
                is_enrolled: true,
                created_at: SystemTime::from(OffsetDateTime::from_unix_timestamp(0).unwrap()),
            }),
        }
    }

    fn create_identity_config_json() -> String {
        r#"{"identifier":"Ifa804b7fca12a19eed206ae180b5b576860ae651","enrollment_status":{"is_enrolled":true,"created_at":{"secs_since_epoch":0,"nanos_since_epoch":0}}}"#.into()
    }
}
