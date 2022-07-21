//! Writing file atomically is hard.  This makes it easier
//!
//! Truncating a renaming files are atomic operations, but writing is
//! not.  Because the Config will have to be read from multiple
//! processes, it's important to keep all updates atomic to prevent
//! race-conditions.
//!
//! To update the configuration call `Config::atomic_update`, which
//! generates an AtomicUpdater.

use crate::config::ConfigValues;
use std::path::PathBuf;
use std::{
    fs::{self, File},
    io::Write,
    sync::{Arc, RwLock},
    time::Duration,
};

/// Takes a version of the Config and persists it to disk
#[must_use]
pub struct AtomicUpdater<V: ConfigValues> {
    config_dir: PathBuf,
    config_name: String,
    inner: Arc<RwLock<V>>,
}

impl<V: ConfigValues> AtomicUpdater<V> {
    /// Create a new atomic updater
    pub fn new(config_dir: PathBuf, config_name: String, inner: Arc<RwLock<V>>) -> Self {
        Self {
            config_dir,
            config_name,
            inner,
        }
    }

    /// Do the thing that we said it was gonna do
    pub fn run(self) -> anyhow::Result<()> {
        let inner = self.inner.read().unwrap();

        let tmp_path = self
            .config_dir
            .join(format!("{}.json.tmp", &self.config_name));
        let regular_path = self.config_dir.join(format!("{}.json", &self.config_name));

        // Repeatedly try to create this file, in case another
        // instance is _also_ trying to currently update the
        // configuration
        let mut new_f = loop {
            match File::create(&tmp_path) {
                Ok(f) => break f,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        };

        // First write the file
        let json: String =
            serde_json::to_string_pretty(&*inner).expect("failed to serialise config");
        new_f
            .write_all(json.as_bytes())
            .expect("failed to write config");

        // Then rename it over the existing config
        fs::rename(&tmp_path, &regular_path)?;

        Ok(())
    }
}
