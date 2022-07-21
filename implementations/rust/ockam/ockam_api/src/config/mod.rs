use crate::config::atomic::AtomicUpdater;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub mod atomic;

pub trait ConfigValues: Serialize + DeserializeOwned {
    fn default_values(config_dir: &Path) -> Self;
}

#[derive(Clone)]
pub struct Config<V: ConfigValues> {
    dir: PathBuf,
    inner: Arc<RwLock<V>>,
}

impl<V: ConfigValues> Config<V> {
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn inner(&self) -> &Arc<RwLock<V>> {
        &self.inner
    }

    /// Attempt to load a config.  If none exists, one is created and then returned.
    pub fn load(dir: PathBuf) -> Self {
        if let Err(e) = create_dir_all(&dir) {
            eprintln!("failed to create configuration directory {:?}: {}", &dir, e);
            std::process::exit(-1);
        }

        let config_path = dir.join("config.json");
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
                let new_inner = V::default_values(&dir);
                let json: String =
                    serde_json::to_string_pretty(&new_inner).expect("failed to serialise config");
                let mut f =
                    File::create(config_path).expect("failed to create default config file");
                f.write_all(json.as_bytes())
                    .expect("failed to write config");
                new_inner
            }
        };

        Self {
            dir,
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Atomically update the configuration
    pub fn atomic_update(&self) -> AtomicUpdater<V> {
        AtomicUpdater::new(self.dir.clone(), "config".to_string(), self.inner.clone())
    }
}
