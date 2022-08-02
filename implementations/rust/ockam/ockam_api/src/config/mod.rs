use crate::config::atomic::AtomicUpdater;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod atomic;
pub mod cli;
pub mod snippet;

pub trait ConfigValues: Serialize + DeserializeOwned {
    fn default_values(config_dir: &Path) -> Self;
}

#[derive(Clone)]
pub struct Config<V: ConfigValues> {
    path: PathBuf,
    inner: Arc<RwLock<V>>,
}

impl<V: ConfigValues> Config<V> {
    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap()
    }

    pub fn inner(&self) -> &Arc<RwLock<V>> {
        &self.inner
    }

    /// Read lock the inner collection and return a guard to it
    pub fn readlock_inner(&self) -> RwLockReadGuard<'_, V> {
        self.inner.read().unwrap()
    }

    /// Write lock the inner collection and return a guard to it
    pub fn writelock_inner(&self) -> RwLockWriteGuard<'_, V> {
        self.inner.write().unwrap()
    }

    /// Attempt to load a config.  If none exists, one is created and then returned.
    pub fn load(config_path: PathBuf) -> Self {
        let dir = config_path
            .parent()
            .expect("Configuration path has no parent directory");

        if let Err(e) = create_dir_all(&dir) {
            eprintln!("failed to create configuration directory {:?}: {}", &dir, e);
            std::process::exit(-1);
        }

        let inner = match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).expect("failed to read config");
                serde_json::from_str(&buf).unwrap_or_else(|_| {
                    panic!(
                        "failed to parse config.  Try deleting {}",
                        config_path.display()
                    )
                })
            }
            Err(_) => {
                let new_inner = V::default_values(dir);
                let json: String =
                    serde_json::to_string_pretty(&new_inner).expect("failed to serialise config");
                let mut f =
                    File::create(&config_path).expect("failed to create default config file");
                f.write_all(json.as_bytes())
                    .expect("failed to write config");
                new_inner
            }
        };

        Self {
            path: config_path,
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Atomically update the configuration
    pub fn atomic_update(&self) -> AtomicUpdater<V> {
        AtomicUpdater::new(
            self.dir().to_path_buf(),
            "config".to_string(),
            self.inner.clone(),
        )
    }
}
