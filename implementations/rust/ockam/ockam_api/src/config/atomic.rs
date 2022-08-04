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
use std::path::{Path, PathBuf};
use std::{
    fs::{self, File},
    io::Write,
    sync::{Arc, RwLock},
    time::Duration,
};

/// Takes a version of the Config and persists it to disk
#[must_use]
pub struct AtomicUpdater<V: ConfigValues> {
    config_path: PathBuf,
    inner: Arc<RwLock<V>>,
}

fn make_tmp_path(p: &Path) -> PathBuf {
    let mut p2 = p.to_path_buf();
    let new_name = format!(
        "{}.tmp",
        p2.file_name()
            .expect("config path ended in '..' -- this is not allowed")
            .to_str()
            .expect("config name was not valid UTF-8")
    );
    p2.set_file_name(new_name);
    p2
}

impl<V: ConfigValues> AtomicUpdater<V> {
    /// Create a new atomic updater
    pub fn new(config_path: PathBuf, inner: Arc<RwLock<V>>) -> Self {
        Self { config_path, inner }
    }

    /// Do the thing that we said it was gonna do
    pub fn run(self) -> anyhow::Result<()> {
        let inner = self.inner.read().unwrap();
        let tmp_path = make_tmp_path(&self.config_path);

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
        fs::rename(&tmp_path, &self.config_path)?;

        Ok(())
    }
}
