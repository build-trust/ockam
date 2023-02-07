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
    async fn get(&self, id: &str, namespace: &str) -> Result<Option<Vec<u8>>> {
        let m = self.map.read().unwrap();
        if let Some(a) = m.get(namespace) {
            return Ok(a.get(id).cloned());
        }
        Ok(None)
    }

    async fn set(&self, id: &str, namespace: String, val: Vec<u8>) -> Result<()> {
        let mut m = self.map.write().unwrap();
        match m.get_mut(&namespace) {
            Some(a) => {
                a.insert(id.to_string(), val);
            }
            None => {
                m.insert(namespace, BTreeMap::from([(id.to_string(), val)]));
            }
        }
        Ok(())
    }

    async fn del(&self, id: &str, namespace: &str) -> Result<()> {
        let mut m = self.map.write().unwrap();
        if let Some(a) = m.get_mut(namespace) {
            a.remove(id);
            if a.is_empty() {
                m.remove(namespace);
            }
        }
        Ok(())
    }

    async fn keys(&self, namespace: &str) -> Result<Vec<String>> {
        Ok(self
            .map
            .read()
            .unwrap()
            .get(namespace)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default())
    }
}
