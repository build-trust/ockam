use super::Storage;
use core::convert::Infallible;
use ockam_core::async_trait;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::{Arc, RwLock};

type Attributes = BTreeMap<String, Vec<u8>>;

#[derive(Clone, Default)]
pub struct Store {
    map: Arc<RwLock<BTreeMap<String, Attributes>>>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            map: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl Storage for Store {
    type Error = Infallible;

    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        let m = self.map.read().unwrap();
        if let Some(a) = m.get(id) {
            return Ok(a.get(key).cloned());
        }
        Ok(None)
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<(), Self::Error> {
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

    async fn del(&self, id: &str, key: &str) -> Result<(), Self::Error> {
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
