use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::atomic::AtomicUpdater;

pub mod atomic;
pub mod cli;
pub mod lookup;

pub trait ConfigValues: Serialize + DeserializeOwned {
    fn default_values(config_dir: &Path) -> Self;
}

#[derive(Clone, Debug)]
pub struct Config<V: ConfigValues> {
    config_dir: PathBuf,
    config_name: String,
    inner: Arc<RwLock<V>>,
}

impl<V: ConfigValues> Config<V> {
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join(&self.config_name)
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn config_name(&self) -> &str {
        &self.config_name
    }

    pub fn inner(&self) -> &Arc<RwLock<V>> {
        &self.inner
    }

    /// Read lock the inner collection and return a guard to it
    pub fn read(&self) -> RwLockReadGuard<'_, V> {
        self.inner.read().unwrap()
    }

    /// Write lock the inner collection and return a guard to it
    pub fn write(&self) -> RwLockWriteGuard<'_, V> {
        self.inner.write().unwrap()
    }

    /// Attempt to load a config.  If none exists, one is created and then returned.
    pub fn load(config_dir: &Path, config_name: &str) -> anyhow::Result<Self> {
        create_dir_all(config_dir)?;

        let config_name = format!("{}.json", config_name);
        let config_path = config_dir.join(&config_name);

        let create_new = || -> anyhow::Result<V> {
            let new_inner = V::default_values(config_dir);
            let json: String =
                serde_json::to_string_pretty(&new_inner).context("failed to serialise config")?;
            let mut f =
                File::create(&config_path).context("failed to create default config file")?;
            f.write_all(json.as_bytes())
                .context("failed to write config")?;
            Ok(new_inner)
        };

        let inner = match File::open(&config_path) {
            Ok(ref mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf)
                    .context("failed to read config")?;
                if buf.is_empty() {
                    create_new()?
                } else {
                    serde_json::from_str(&buf).unwrap_or_else(|_| {
                        panic!(
                            "Failed to parse config.  Try deleting {}",
                            config_path.display()
                        )
                    })
                }
            }
            Err(_) => create_new()?,
        };

        Ok(Self {
            config_dir: config_dir.to_path_buf(),
            config_name,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    /// Atomically update the configuration
    pub fn persist_config_updates(&self) -> anyhow::Result<()> {
        AtomicUpdater::new(self.config_dir.join(&self.config_name), self.inner.clone()).run()
    }
}
