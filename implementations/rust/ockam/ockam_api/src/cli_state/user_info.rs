use super::Result;
use crate::cloud::enroll::auth0::UserInfo;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UsersInfoState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserInfoState {
    path: PathBuf,
    config: UserInfoConfig,
}

impl Display for UserInfoState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Email: {}", self.config.email)?;
        Ok(())
    }
}

type UserInfoConfig = UserInfo;

mod traits {
    use super::*;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateDirTrait for UsersInfoState {
        type Item = UserInfoState;
        const DEFAULT_FILENAME: &'static str = "user_info";
        const DIR_NAME: &'static str = "users_info";
        const HAS_DATA_DIR: bool = false;

        fn new(root_path: &Path) -> Self {
            Self {
                dir: Self::build_dir(root_path),
            }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }
    }

    #[async_trait]
    impl StateItemTrait for UserInfoState {
        type Config = UserInfoConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            Ok(Self { path, config })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            Ok(Self { path, config })
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
