use crate::error::{Error, FfiOckamError};
use std::sync::{Arc, RwLock};

struct NoMutexObject<T> {
    handle: u64,
    object: Arc<T>,
}

struct NoMutexStorage<T> {
    vec: Vec<NoMutexObject<T>>,
    next_id: u64,
}

impl<T> Default for NoMutexStorage<T> {
    fn default() -> Self {
        Self {
            vec: Vec::new(),
            next_id: 0,
        }
    }
}

/// FfiObjectStorage
pub struct FfiObjectNoMutexStorage<T> {
    storage: RwLock<NoMutexStorage<T>>,
}

impl<T> FfiObjectNoMutexStorage<T> {
    /// Remove object
    pub fn remove_object(&self, handle: u64) -> Result<T, FfiOckamError> {
        let mut storage = self.storage.write().unwrap();

        let index = storage
            .vec
            .iter()
            .position(|x| x.handle == handle)
            .ok_or(Error::EntryNotFound)?;

        let item = storage.vec.remove(index);
        let item = Arc::try_unwrap(item.object)
            .ok()
            .ok_or(Error::OwnershipError)?;

        Ok(item)
    }

    /// Insert object
    pub fn insert_object(&self, object: T) -> Result<u64, FfiOckamError> {
        let mut storage = self.storage.write().unwrap();

        storage.next_id += 1;
        let handle = storage.next_id;
        storage.vec.push(NoMutexObject {
            handle,
            object: Arc::new(object),
        });

        Ok(handle)
    }

    /// Get object
    pub fn get_object(&self, handle: u64) -> Result<Arc<T>, FfiOckamError> {
        let storage = self.storage.read().unwrap();

        let item = storage
            .vec
            .iter()
            .find(|&x| x.handle == handle)
            .ok_or(Error::EntryNotFound)?;

        Ok(item.object.clone())
    }
}

impl<T> Default for FfiObjectNoMutexStorage<T> {
    fn default() -> Self {
        Self {
            storage: RwLock::new(NoMutexStorage::default()),
        }
    }
}
