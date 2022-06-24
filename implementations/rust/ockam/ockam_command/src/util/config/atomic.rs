//! Writing file atomically is hard.  This makes it easier
//!
//! Truncating a renaming files are atomic operations, but writing is
//! not.  Because the OckamConfig will have to be read from multiple
//! processes, it's important to keep all updates atomic to prevent
//! race-conditions.
//!
//! To update the configuration call `Config::atomic_update`, which
//! generates an AtomicUpdater.

use directories::ProjectDirs;

use crate::util::SyncConfig;
use std::{
    fs::{self, File},
    io::Write,
    sync::{Arc, RwLock},
    time::Duration,
};

/// Takes a version of the OckamConfig and persists it to disk
#[must_use]
pub struct AtomicUpdater {
    dirs: ProjectDirs,
    inner: Arc<RwLock<SyncConfig>>,
}

impl AtomicUpdater {
    /// Create a new atomic updater
    pub fn new(dirs: ProjectDirs, inner: Arc<RwLock<SyncConfig>>) -> Self {
        Self { dirs, inner }
    }

    /// Do the thing that we said it was gonna do
    pub fn run(self) -> anyhow::Result<()> {
        let inner = self.inner.read().unwrap();

        let cfg_dir = self.dirs.config_dir();

        // Repeatedly try to create this file, in case another
        // instance is _also_ trying to currently update the
        // configuration
        let mut new_f = loop {
            match File::create(cfg_dir.join("__temp.cfg")) {
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
        fs::rename(cfg_dir.join("__temp.cfg"), cfg_dir.join("config.json"))?;

        Ok(())
    }
}
