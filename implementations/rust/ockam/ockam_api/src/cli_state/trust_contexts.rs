use super::Result;
use crate::config::cli::TrustContextConfig;
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

mod traits {
    use super::*;
    use crate::cli_state::file_stem;
    use crate::cli_state::traits::*;
    use ockam_core::async_trait;
    use std::path::Path;

    #[async_trait]
    impl StateDirTrait for TrustContextsState {
        type Item = TrustContextState;

        fn new(dir: PathBuf) -> Self {
            Self { dir }
        }

        fn default_filename() -> &'static str {
            "trust_context"
        }

        fn build_dir(root_path: &Path) -> PathBuf {
            root_path.join("trust_contexts")
        }

        fn has_data_dir() -> bool {
            false
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

        fn name(&self) -> &str {
            &self.name
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }

        fn data_path(&self) -> Option<&PathBuf> {
            unreachable!()
        }

        fn config(&self) -> &Self::Config {
            &self.config
        }
    }
}
