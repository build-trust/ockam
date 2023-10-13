use crate::state::model::ModelState;
use miette::miette;
use ockam::identity::storage::Storage;
use ockam::LmdbStorage;
use ockam_core::async_trait;
use std::path::Path;

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
        Ok(())
    }

    async fn load(&self) -> Result<Option<ModelState>> {
        match self.storage.get(MODEL_STATE_ID, MODEL_STATE_KEY).await {
            Err(e) => Err(miette!(e).into()),
            Ok(None) => Ok(None),
            Ok(Some(bytes)) => {
                Ok(serde_json::from_slice(bytes.as_slice()).map_err(|e| miette!(e))?)
            }
        }
    }
}
