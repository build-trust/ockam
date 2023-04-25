use crate::storage::value_storage::ValueStorage;
use crate::storage::value_storage_error::ValueStorageError;
use crate::tokio::task::{self, JoinError};
use cfg_if::cfg_if;
use fs2::FileExt; //locking
use ockam_core::compat::boxed::Box;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// File Storage
/* There are three files involved
 * - The actual vault file
 * - A temp file used to avoid data lost during writes:  vault is entirely
 *   written to the temp file, then file renamed.
 * - A "lock" file.  It's used to control inter-process access to the vault.
 *   Before reading or writing to the vault, first need to get a shared or exclusive lock
 *   on this file.  We don't lock over the vault file directly, because doesn't play well with
 *   the file rename we do */
#[derive(Clone)]
pub struct FileValueStorage<V, R> {
    path: PathBuf,
    temp_path: PathBuf,
    lock_path: PathBuf,
    _phantom_data: PhantomData<(V, R)>,
}

impl<V: Default + Serialize + for<'de> Deserialize<'de>, R> FileValueStorage<V, R> {
    /// Create and init Storage
    pub async fn create(path: PathBuf) -> Result<Self> {
        let mut s = Self::new(path);
        s.init().await?;
        Ok(s)
    }

    /// Create the file storage but don't initialize it
    fn new(path: PathBuf) -> Self {
        let temp_path = Self::path_with_suffix(&path, ".tmp");
        let lock_path = Self::path_with_suffix(&path, ".lock");
        Self {
            path,
            temp_path,
            lock_path,
            _phantom_data: PhantomData,
        }
    }

    /// Create FileStorage using file at given Path
    /// If file doesn't exist, it will be created
    async fn init(&mut self) -> Result<()> {
        // This can block, but only when first initializing and just need to write an empty vault.
        // So didn't bother to do it async
        let lock_file = Self::open_lock_file(&self.lock_path)?;
        lock_file.lock_exclusive().map_err(map_io_err)?;
        if !self.path.exists() {
            let empty = V::default();
            Self::flush_to_file(&self.path, &self.temp_path, &empty)?;
        }
        lock_file.unlock().map_err(map_io_err)?;
        Ok(())
    }

    fn load(path: &PathBuf) -> Result<V> {
        let file = File::open(path).map_err(map_io_err)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader::<BufReader<File>, V>(reader)
            .map_err(|e| ValueStorageError::InvalidStorageData(e.to_string()))?)
    }

    // Flush vault to target, using temp_path as intermediary file.
    fn flush_to_file(target: &PathBuf, temp_path: &PathBuf, value: &V) -> Result<()> {
        let data = serde_json::to_vec(value).map_err(|_| ValueStorageError::StorageError)?;
        use std::io::prelude::*;
        cfg_if! {
            if #[cfg(windows)] {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(temp_path)
                    .map_err(|_| ValueStorageError::StorageError)?;
            } else {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .mode(0o600)
                    .open(temp_path)
                    .map_err(|_| ValueStorageError::StorageError)?;
            }
        }
        file.write_all(&data)
            .map_err(|_| ValueStorageError::StorageError)?;
        file.flush().map_err(|_| ValueStorageError::StorageError)?;
        file.sync_all()
            .map_err(|_| ValueStorageError::StorageError)?;
        std::fs::rename(temp_path, target).map_err(|_| ValueStorageError::StorageError)?;
        Ok(())
    }
}

impl<V, R> FileValueStorage<V, R> {
    fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
        match path.extension() {
            None => path.with_extension(suffix),
            Some(e) => path.with_extension(format!("{}{}", e.to_str().unwrap(), suffix)),
        }
    }

    fn open_lock_file(lock_path: &PathBuf) -> Result<File> {
        std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(lock_path)
            .map_err(map_io_err)
    }
}

#[async_trait]
impl<
        V: Default + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
        R: Send + Sync + 'static,
    > ValueStorage<V, R> for FileValueStorage<V, R>
{
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()> {
        let lock_path = self.lock_path.clone();
        let temp_path = self.temp_path.clone();
        let path = self.path.clone();
        let tr = move || -> Result<()> {
            let file = FileValueStorage::<V, R>::open_lock_file(&lock_path)?;
            file.lock_exclusive().map_err(map_io_err)?;
            let existing_value = FileValueStorage::<V, R>::load(&path)?;
            let updated_value = f(existing_value)?;
            FileValueStorage::<V, R>::flush_to_file(&path, &temp_path, &updated_value)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(map_io_err)?;
            Ok(())
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }

    async fn modify_value(
        &self,
        f: impl Fn(V) -> Result<(V, R)> + Send + Sync + 'static,
    ) -> Result<R> {
        let lock_path = self.lock_path.clone();
        let temp_path = self.temp_path.clone();
        let path = self.path.clone();
        let tr = move || -> Result<R> {
            let file = FileValueStorage::<V, R>::open_lock_file(&lock_path)?;
            file.lock_exclusive().map_err(map_io_err)?;
            let existing_value = FileValueStorage::<V, R>::load(&path)?;
            let (updated_value, result) = f(existing_value)?;
            FileValueStorage::<V, R>::flush_to_file(&path, &temp_path, &updated_value)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(map_io_err)?;
            Ok(result)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }

    async fn read_value(&self, f: impl Fn(V) -> Result<R> + Send + Sync + 'static) -> Result<R> {
        let path = self.path.clone();
        let lock_path = self.lock_path.clone();
        let tr = move || {
            let file = FileValueStorage::<V, R>::open_lock_file(&lock_path)?;
            file.lock_shared().map_err(map_io_err)?;
            let data = FileValueStorage::<V, R>::load(&path)?;
            let r = f(data)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(map_io_err)?;
            Ok(r)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_io_err(err: std::io::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::compat::rand::{thread_rng, RngCore};
    use ockam_core::Result;

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn test_vault_synchronization() -> Result<()> {
        let dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let file_name = hex::encode(bytes);

        let storage: FileValueStorage<Value, Value> =
            FileValueStorage::create(dir.join(file_name)).await.unwrap();

        let initial = storage.read_value(move |value: Value| Ok(value)).await?;

        // sanity check
        assert_eq!(Value::default(), Value(0));

        // the initial value is the default value
        assert_eq!(initial, Value::default());

        // the value can be updated
        storage
            .update_value(move |_: Value| Ok(Value(10)))
            .await
            .unwrap();

        // the new value can be read again
        let updated = storage.read_value(move |value: Value| Ok(value)).await?;
        assert_eq!(updated, Value(10));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
    struct Value(u8);
}
