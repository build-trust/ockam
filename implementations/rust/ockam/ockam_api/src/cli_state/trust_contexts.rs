use super::Result;
use crate::config::cli::TrustContextConfig;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TrustContextsState {
    dir: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TrustContextState {
    name: String,
    path: PathBuf,
    config: TrustContextConfig,
}

impl TrustContextState {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Display for TrustContextState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        Ok(())
    }
}

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;

    #[async_trait]
    impl StateDirTrait for TrustContextsState {
        type Item = TrustContextState;
        const DEFAULT_FILENAME: &'static str = "trust_context";
        const DIR_NAME: &'static str = "trust_contexts";
        const HAS_DATA_DIR: bool = false;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn dir(&self) -> &PathBuf {
            &self.dir
        }
    }

    #[async_trait]
    impl StateItemTrait for TrustContextState {
        type Config = TrustContextConfig;

        fn new(path: PathBuf, config: Self::Config) -> Result<Self> {
            let contents = serde_json::to_string(&config)?;
            std::fs::write(&path, contents)?;
            let name = file_stem(&path)?;
            Ok(Self { name, path, config })
        }

        fn load(path: PathBuf) -> Result<Self> {
            let name = file_stem(&path)?;
            let contents = std::fs::read_to_string(&path)?;
            let config = serde_json::from_str(&contents)?;
            Ok(Self { name, path, config })
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
