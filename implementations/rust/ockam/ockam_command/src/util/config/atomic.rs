//! Writing file atomically is hard.  This makes it easier
//!
//! Truncating a renaming files are atomic operations, but writing is
//! not.  Because the OckamConfig will have to be read from multiple
//! processes, it's important to keep all updates atomic to prevent
//! race-conditions.
//!
//! To update the configuration call `Config::atomic_update`, which
//! generates an AtomicUpdater.

use crate::util::OckamConfig;
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempfile_in;

/// Takes a version of the OckamConfig and persists it to disk
#[must_use]
pub struct AtomicUpdater<'cfg> {
    inner: &'cfg OckamConfig,
}

impl<'cfg> AtomicUpdater<'cfg> {
    /// Create a new atomic updater
    pub fn new(inner: &'cfg OckamConfig) -> Self {
        Self { inner }
    }

    /// Do the thing that we said it was gonna do
    pub fn run(self) -> anyhow::Result<()> {
        let cfg_dir = self.inner.dirs.config_dir();

        let mut new_f = File::create(cfg_dir.join("__temp.cfg"))?;

        // First write the file
        let json: String =
            serde_json::to_string_pretty(self.inner).expect("failed to serialise config");
        new_f
            .write_all(json.as_bytes())
            .expect("failed to write config");

        // Then rename it over the existing config
        fs::rename(cfg_dir.join("__temp.cfg"), cfg_dir.join("config.json"))?;

        Ok(())
    }
}
