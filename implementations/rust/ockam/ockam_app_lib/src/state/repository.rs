use crate::state::model::ModelState;
use miette::miette;
use ockam::identity::storage::Storage;
use ockam::LmdbStorage;
use ockam_core::async_trait;
use std::path::Path;
use tracing::trace;

use crate::Result;

const MODEL_STATE_ID: &str = "model_state";
const MODEL_STATE_KEY: &str = "model_state_key";

/// The ModelStateRepository is responsible for storing and loading
/// ModelState data (user information, shared services etc...)
/// The state must be stored everytime it is modified (see set_user_info in AppState for example)
/// so that it can be loaded again when the application starts up
#[async_trait]
pub trait ModelStateRepository: Send + Sync + 'static {
    async fn store(&self, model_state: &ModelState) -> Result<()>;
    async fn load(&self) -> Result<Option<ModelState>>;
}

/// This implementation of the ModelStateRepository piggy-backs for now on the LMDB storage
/// which is used to store all the data related to identities.
/// We will possibly store all data eventually using SQLite and in that case the ModelData
/// can be a set of tables dedicated to the desktop application
pub struct LmdbModelStateRepository {
    storage: LmdbStorage,
}

impl LmdbModelStateRepository {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            storage: LmdbStorage::new(path).await.map_err(|e| miette!(e))?,
        })
    }
}

/// The implementation simply serializes / deserializes the ModelState as JSON
#[async_trait]
impl ModelStateRepository for LmdbModelStateRepository {
    async fn store(&self, model_state: &ModelState) -> Result<()> {
        self.storage
            .set(
                MODEL_STATE_ID,
                MODEL_STATE_KEY.to_string(),
                serde_json::to_vec(model_state)?,
            )
            .await
            .map_err(|e| miette!(e))?;
        trace!(?model_state, "stored model state");
        Ok(())
    }

    async fn load(&self) -> Result<Option<ModelState>> {
        match self.storage.get(MODEL_STATE_ID, MODEL_STATE_KEY).await {
            Err(e) => Err(miette!(e).into()),
            Ok(None) => Ok(None),
            Ok(Some(bytes)) => {
                let state = serde_json::from_slice(bytes.as_slice()).map_err(|e| miette!(e))?;
                trace!(?state, "loaded model state");
                Ok(state)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::nodes::models::portal::OutletStatus;
    use ockam_core::Address;

    #[tokio::test]
    async fn store_and_load_tcp_outlets() {
        let path = std::env::temp_dir().join("ockam_app_lib_test");
        let _ = std::fs::remove_dir_all(&path);

        // Initial state
        let repo = LmdbModelStateRepository::new(&path).await.unwrap();
        let mut state = ModelState::default();
        repo.store(&state).await.unwrap();
        let loaded = repo.load().await.unwrap().unwrap();
        assert!(state.tcp_outlets.is_empty());
        assert_eq!(state, loaded);

        // Add a tcp outlet
        state.add_tcp_outlet(OutletStatus::new(
            "127.0.0.1:1001".parse().unwrap(),
            Address::from_string("s1"),
            "s1",
            None,
        ));
        repo.store(&state).await.unwrap();
        let loaded = repo.load().await.unwrap().unwrap();
        assert_eq!(state.tcp_outlets.len(), 1);
        assert_eq!(state, loaded);

        // Add a few more
        for i in 2..=5 {
            state.add_tcp_outlet(OutletStatus::new(
                format!("127.0.0.1:100{i}").parse().unwrap(),
                Address::from_string(format!("s{i}")),
                &format!("s{i}"),
                None,
            ));
            repo.store(&state).await.unwrap();
        }
        let loaded = repo.load().await.unwrap().unwrap();
        assert_eq!(state.tcp_outlets.len(), 5);
        assert_eq!(state, loaded);

        // Reload from DB scratch to emulate an app restart
        let repo = LmdbModelStateRepository::new(&path).await.unwrap();
        let loaded = repo.load().await.unwrap().unwrap();
        assert_eq!(state.tcp_outlets.len(), 5);
        assert_eq!(state, loaded);
    }
}
