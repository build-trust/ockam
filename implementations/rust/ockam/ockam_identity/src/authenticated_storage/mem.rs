use super::AuthenticatedStorage;
use ockam_core::async_trait;
use ockam_core::compat::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, RwLock},
    vec::Vec,
};
use ockam_core::Result;

type Attributes = BTreeMap<String, Vec<u8>>;

/// Non-persistent table stored in RAM
#[derive(Clone, Default)]
pub struct InMemoryStorage {
    map: Arc<RwLock<BTreeMap<String, Attributes>>>,
}

impl InMemoryStorage {
    /// Constructor
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl AuthenticatedStorage for InMemoryStorage {
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let m = self.map.read().unwrap();
        if let Some(a) = m.get(id) {
            return Ok(a.get(key).cloned());
        }
        Ok(None)
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()> {
        let mut m = self.map.write().unwrap();
        match m.get_mut(id) {
            Some(a) => {
                a.insert(key, val);
            }
            None => {
                m.insert(id.to_string(), BTreeMap::from([(key, val)]));
            }
        }
        Ok(())
    }

    async fn del(&self, id: &str, key: &str) -> Result<()> {
        let mut m = self.map.write().unwrap();
        if let Some(a) = m.get_mut(id) {
            a.remove(key);
            if a.is_empty() {
                m.remove(id);
            }
        }
        Ok(())
    }
}
