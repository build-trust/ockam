use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

use ockam_identity::{
    IdentitiesRepository, IdentitiesStorage, Identity, IdentityChangeHistory, IdentityIdentifier,
    LmdbStorage,
};

use crate::cli_state::traits::{StateDirTrait, StateItemTrait};
use crate::cli_state::CliStateError;

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

    pub fn get_by_identifier(&self, identifier: &IdentityIdentifier) -> Result<IdentityState> {
        let identities = self.list()?;

        let identity_state = identities
            .into_iter()
            .find(|ident_state| &ident_state.config.identifier() == identifier);

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityState {
    name: String,
    path: PathBuf,
    /// The path to the directory containing the authenticated storage files, shared amongst all identities
    data_path: PathBuf,
    config: IdentityConfig,
}

impl IdentityState {
    pub fn identifier(&self) -> IdentityIdentifier {
        self.config.identifier()
    }

    pub fn set_enrollment_status(&mut self) -> Result<()> {
        self.config.enrollment_status = Some(EnrollmentStatus::enrolled());
        self.persist()
    }

    fn build_data_path(path: &Path) -> PathBuf {
        path.parent().expect("Should have parent").join("data")
    }

    pub fn name(&self) -> &str {
        &self.name
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
    pub identifier: IdentityIdentifier,
    pub enrollment_status: Option<EnrollmentStatus>,
}

impl PartialEq for IdentityConfig {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
    }
}

impl Eq for IdentityConfig {}

impl IdentityConfig {
    pub async fn new(identifier: &IdentityIdentifier) -> Self {
        Self {
            identifier: identifier.clone(),
            enrollment_status: None,
        }
    }

    pub fn identifier(&self) -> IdentityIdentifier {
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

#[derive(Deserialize, Debug, Clone)]
struct IdentityConfigV1 {
    identifier: IdentityIdentifier,
    #[allow(dead_code)]
    change_history: IdentityChangeHistory,
    enrollment_status: Option<EnrollmentStatus>,
}

#[derive(Deserialize, Debug, Clone)]
struct IdentityConfigV2 {
    identity: Identity,
    enrollment_status: Option<EnrollmentStatus>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum IdentityConfigs {
    V1(IdentityConfigV1),
    V2(IdentityConfigV2),
    V3(IdentityConfig),
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

        fn new(dir: PathBuf) -> Self {
            Self { dir }
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
            let contents = match std::fs::read_to_string(&path).ok() {
                Some(contents) => contents,
                None => return Ok(()),
            };

            // read the configuration and migrate to the most recent format if an old format is found
            // the most recent configuration only contains an identity identifier, so if we find an
            // old format we store the full identity in the shared identities repository before
            // writing the most recent configuration format
            match serde_json::from_str(&contents)? {
                IdentityConfigs::V1(config) => {
                    let identifier = config.identifier.clone();
                    let new_config = IdentityConfig {
                        identifier: identifier.clone(),
                        enrollment_status: config.enrollment_status,
                    };
                    let identity = Identity::new(identifier, config.change_history);
                    self.identities_repository()
                        .await?
                        .update_identity(&identity)
                        .await?;
                    std::fs::write(&path, serde_json::to_string(&new_config)?)?;
                }
                IdentityConfigs::V2(config) => {
                    let new_config = IdentityConfig {
                        identifier: config.identity.identifier(),
                        enrollment_status: config.enrollment_status,
                    };
                    self.identities_repository()
                        .await?
                        .update_identity(&config.identity)
                        .await?;
                    std::fs::write(&path, serde_json::to_string(&new_config)?)?;
                }
                IdentityConfigs::V3(_) => (),
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
    use core::str::FromStr;

    use ockam_identity::IdentityChangeHistory;

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

    #[test]
    fn test_deserialize_legacy() {
        let json = create_identity_config_json_legacy();
        let actual: IdentityConfig = serde_json::from_str(json.as_str()).unwrap();
        let expected = create_identity_config();
        assert_eq!(actual, expected)
    }

    fn create_identity_config() -> IdentityConfig {
        let data = hex::decode("0144c7eb72dd1e633f38e0d0521e9d5eb5072f6418176529eb1b00189e4d69ad2e000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020c6c52380125d42b0b4da922b1cff8503a258c3497ec8ac0b4a3baa0d9ca7b3780301014075064b902bda9d16db81ab5f38fbcf226a0e904e517a8c087d379ea139df1f2d7fee484ac7e1c2b7ab2da75f85adef6af7ddb05e7fa8faf180820cb9e86def02").unwrap();
        let identity = Identity::new(
            IdentityIdentifier::from_str(
                "Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50",
            )
            .unwrap(),
            IdentityChangeHistory::import(data.to_vec().as_slice()).unwrap(),
        );
        IdentityConfig {
            identifier: identity.identifier(),
            enrollment_status: Some(EnrollmentStatus {
                is_enrolled: true,
                created_at: SystemTime::from(OffsetDateTime::from_unix_timestamp(0).unwrap()),
            }),
        }
    }

    fn create_identity_config_json() -> String {
        r#"{"identifier":"Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50","enrollment_status":{"is_enrolled":true,"created_at":{"secs_since_epoch":0,"nanos_since_epoch":0}}}"#.into()
    }

    fn create_identity_config_json_legacy() -> String {
        r#"{"identifier":"Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50","change_history":[{"identifier":[68,199,235,114,221,30,99,63,56,224,208,82,30,157,94,181,7,47,100,24,23,101,41,235,27,0,24,158,77,105,173,46],"change":{"CreateKey":{"prev_change_id":[5,71,201,50,57,186,61,129,142,194,108,156,218,221,42,53,203,223,31,163,182,209,167,49,224,97,100,177,7,159,183,184],"key_attributes":{"label":"OCKAM_RK","secret_attributes":{"stype":"Ed25519","persistence":"Persistent","length":32}},"public_key":{"data":[198,197,35,128,18,93,66,176,180,218,146,43,28,255,133,3,162,88,195,73,126,200,172,11,74,59,170,13,156,167,179,120],"stype":"Ed25519"}}},"signatures":[{"stype":"SelfSign","data":[117,6,75,144,43,218,157,22,219,129,171,95,56,251,207,34,106,14,144,78,81,122,140,8,125,55,158,161,57,223,31,45,127,238,72,74,199,225,194,183,171,45,167,95,133,173,239,106,247,221,176,94,127,168,250,241,128,130,12,185,232,109,239,2]}]}],"enrollment_status":{"is_enrolled":true,"created_at":{"secs_since_epoch":0,"nanos_since_epoch":0}}}"#.into()
    }
}
