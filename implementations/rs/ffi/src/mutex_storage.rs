use crate::error::{Error, FfiOckamError};
use std::sync::{Arc, Mutex, RwLock};

struct MutexObject<T: ?Sized> {
    handle: u64,
    object: Arc<Mutex<T>>,
}

struct MutexStorage<T: ?Sized> {
    vec: Vec<MutexObject<T>>,
    next_id: u64,
}

impl<T: ?Sized> Default for MutexStorage<T> {
    fn default() -> Self {
        Self {
            vec: Vec::new(),
            next_id: 0,
        }
    }
}

/// FfiObjectStorage
pub struct FfiObjectMutexStorage<T: ?Sized> {
    storage: RwLock<MutexStorage<T>>,
}

impl<T> FfiObjectMutexStorage<T> {
    /// Remove object
    pub fn remove_object_sized(&self, handle: u64) -> Result<T, FfiOckamError> {
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

        Ok(item.into_inner().unwrap())
    }

    /// Insert object
    pub fn insert_object_sized(&self, object: T) -> Result<u64, FfiOckamError> {
        let mut storage = self.storage.write().unwrap();

        storage.next_id += 1;
        let handle = storage.next_id;
        storage.vec.push(MutexObject {
            handle,
            object: Arc::new(Mutex::new(object)),
        });

        Ok(handle)
    }
}

impl<T: ?Sized> FfiObjectMutexStorage<T> {
    /// Remove object
    pub fn remove_object(&self, handle: u64) -> Result<(), FfiOckamError> {
        let mut storage = self.storage.write().unwrap();

        let index = storage
            .vec
            .iter()
            .position(|x| x.handle == handle)
            .ok_or(Error::EntryNotFound)?;

        let _ = storage.vec.remove(index);

        Ok(())
    }

    /// Insert object
    pub fn insert_object(&self, object: Arc<Mutex<T>>) -> Result<u64, FfiOckamError> {
        let mut storage = self.storage.write().unwrap();

        storage.next_id += 1;
        let handle = storage.next_id;
        storage.vec.push(MutexObject { handle, object });

        Ok(handle)
    }

    /// Get object
    pub fn get_object(&self, handle: u64) -> Result<Arc<Mutex<T>>, FfiOckamError> {
        let storage = self.storage.read().unwrap();

        let item = storage
            .vec
            .iter()
            .find(|&x| x.handle == handle)
            .ok_or(Error::EntryNotFound)?;

        Ok(item.object.clone())
    }
}

impl<T: ?Sized> Default for FfiObjectMutexStorage<T> {
    fn default() -> Self {
        Self {
            storage: RwLock::new(MutexStorage::default()),
        }
    }
}
