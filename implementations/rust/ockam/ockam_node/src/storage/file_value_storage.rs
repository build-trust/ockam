use crate::storage::value_storage::ValueStorage;
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
/// There three files involved
///  - the actual file storing data
///
///  - a temporary file used to avoid losing data during writes. The whole data file is copied to the
///    temporary file then the temporary file is renamed
///
///  - a lock file.  It's used to control inter-process accesses to the data.
///    Before reading or writing to the data fil, a shared or exclusive lock is first acquired
///    on this file.  We don't lock over the data file directly, because doesn't play well with
///    the file rename we do
#[derive(Clone)]
pub struct FileValueStorage<V> {
    path: Box<Path>,
    temp_path: Box<Path>,
    lock_path: Box<Path>,
    _phantom_data: PhantomData<V>,
}

impl<V: Default + Serialize + for<'de> Deserialize<'de>> FileValueStorage<V> {
    /// Create and init the file storage
    pub async fn create(path: &Path) -> Result<Self> {
        let mut s = Self::new(path);
        s.init().await?;
        Ok(s)
    }

    /// Create the file storage but don't initialize it
    fn new(path: &Path) -> Self {
        let temp_path = Self::path_with_suffix(path, "tmp");
        let lock_path = Self::path_with_suffix(path, "lock");
        Self {
            path: path.into(),
            temp_path: temp_path.into(),
            lock_path: lock_path.into(),
            _phantom_data: PhantomData,
        }
    }

    /// Create FileStorage using file at given Path
    /// If file doesn't exist, it will be created
    async fn init(&mut self) -> Result<()> {
        std::fs::create_dir_all(self.path.parent().unwrap())
            .map_err(|e| Error::new(Origin::Node, Kind::Io, e))?;
        // This can block, but only when first initializing and just need to write an empty vault.
        // So didn't bother to do it async
        let lock_file = Self::open_lock_file(&self.lock_path)?;
        lock_file
            .lock_exclusive()
            .map_err(|e| map_io_err(&self.lock_path, e))?;

        let should_flush_default = if self.path.exists() {
            let metadata = self
                .path
                .metadata()
                .map_err(|e| map_io_err(&self.path, e))?;

            metadata.len() == 0
        } else {
            true
        };

        if should_flush_default {
            let empty = V::default();
            Self::flush_to_file(&self.path, &self.temp_path, &empty)?;
        }
        lock_file
            .unlock()
            .map_err(|e| map_io_err(&self.lock_path, e))?;
        Ok(())
    }

    fn load(path: &Path) -> Result<V> {
        let file = File::open(path).map_err(|e| map_io_err(path, e))?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader::<BufReader<File>, V>(reader)
            .map_err(|e| ValueStorageError::InvalidStorageData(e.to_string()))?)
    }

    // Flush vault to target, using temp_path as intermediary file.
    fn flush_to_file(target: &Path, temp_path: &Path, value: &V) -> Result<()> {
        let data = serde_json::to_vec(value)
            .map_err(|e| ValueStorageError::InvalidStorageData(e.to_string()))?;
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

impl<V> FileValueStorage<V> {
    fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
        match path.extension() {
            None => path.with_extension(suffix),
            Some(e) => path.with_extension(format!("{}.{}", e.to_str().unwrap(), suffix)),
        }
    }

    fn open_lock_file(lock_path: &Path) -> Result<File> {
        std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(lock_path)
            .map_err(|e| map_io_err(lock_path, e))
    }
}

#[async_trait]
impl<V: Default + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static> ValueStorage<V>
    for FileValueStorage<V>
{
    async fn update_value(&self, f: impl Fn(V) -> Result<V> + Send + Sync + 'static) -> Result<()> {
        let f = move |v: V| Ok((f(v)?, ()));
        let _ = self.modify_value(f).await?;
        Ok(())
    }

    async fn modify_value<R: Send + Sync + 'static>(
        &self,
        f: impl Fn(V) -> Result<(V, R)> + Send + Sync + 'static,
    ) -> Result<R> {
        let lock_path = self.lock_path.clone();
        let temp_path = self.temp_path.clone();
        let path = self.path.clone();
        let tr = move || -> Result<R> {
            let file = FileValueStorage::<V>::open_lock_file(&lock_path)?;
            file.lock_exclusive().map_err(|e| map_io_err(&path, e))?;
            let existing_value = FileValueStorage::<V>::load(&path)?;
            let (updated_value, result) = f(existing_value)?;
            FileValueStorage::<V>::flush_to_file(&path, &temp_path, &updated_value)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(|e| map_io_err(&path, e))?;
            Ok(result)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }

    async fn read_value<R: Send + Sync + 'static>(
        &self,
        f: impl Fn(V) -> Result<R> + Send + Sync + 'static,
    ) -> Result<R> {
        let path = self.path.clone();
        let lock_path = self.lock_path.clone();
        let tr = move || {
            let file = FileValueStorage::<V>::open_lock_file(&lock_path)?;
            file.lock_shared().map_err(|e| map_io_err(&path, e))?;
            let data = FileValueStorage::<V>::load(&path)?;
            let r = f(data)?;
            // if something goes wrong it will be unlocked once the file handler get closed anyway
            file.unlock().map_err(|e| map_io_err(&path, e))?;
            Ok(r)
        };
        task::spawn_blocking(tr).await.map_err(map_join_err)?
    }
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_io_err(path: &Path, err: std::io::Error) -> Error {
    Error::new(
        Origin::Application,
        Kind::Io,
        format!("{err} for path {:?}", path),
    )
}

/// Represents the failures that can occur when storing values
#[derive(Clone, Debug)]
pub enum ValueStorageError {
    /// IO error
    StorageError,
    /// Invalid Storage data
    InvalidStorageData(String),
}

impl ockam_core::compat::error::Error for ValueStorageError {}

impl core::fmt::Display for ValueStorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StorageError => write!(f, "invalid storage"),
            Self::InvalidStorageData(e) => write!(f, "invalid storage data {:?}", e),
        }
    }
}

impl From<ValueStorageError> for Error {
    #[track_caller]
    fn from(err: ValueStorageError) -> Self {
        Error::new(Origin::Vault, Kind::Invalid, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::compat::rand::{thread_rng, Rng};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_empty_file() -> Result<()> {
        let path = NamedTempFile::new().unwrap();

        let storage = FileValueStorage::<Value>::create(path.path())
            .await
            .unwrap();

        storage.update_value(Ok).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_file_value_storage() -> Result<()> {
        let file_name = hex::encode(thread_rng().gen::<[u8; 8]>());

        let path = tempfile::tempdir()
            .unwrap()
            .into_path()
            .with_file_name(file_name);

        let storage = FileValueStorage::<Value>::create(path.as_path())
            .await
            .unwrap();

        let initial = storage.read_value(Ok).await?;

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
        let updated = storage.read_value(Ok).await?;
        assert_eq!(updated, Value(10));

        Ok(())
    }

    #[derive(Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
    struct Value(u8);
}
