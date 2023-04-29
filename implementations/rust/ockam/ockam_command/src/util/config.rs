//! Handle local node configuration

use std::{ops::Deref, sync::RwLockReadGuard};

use crate::Result;
use tracing::trace;

use ockam_api::config::lookup::ProjectLookup;
use ockam_api::config::{cli, lookup::ConfigLookup, Config};

/// A simple wrapper around the main configuration structure to add
/// local config utility/ query functions
#[derive(Clone)]
pub struct OckamConfig {
    inner: Config<cli::OckamConfig>,
}

impl Deref for OckamConfig {
    type Target = Config<cli::OckamConfig>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl OckamConfig {
    pub fn load() -> Result<OckamConfig> {
        let dir = cli::OckamConfig::dir();
        let inner = Config::<cli::OckamConfig>::load(&dir, "config")?;
        inner.write().dir = Some(dir);
        Ok(Self { inner })
    }

    /// Get read access to the inner raw configuration
    pub fn inner(&self) -> RwLockReadGuard<'_, cli::OckamConfig> {
        self.inner.read()
    }

    /// Get a lookup table
    pub fn lookup(&self) -> ConfigLookup {
        self.inner().lookup().clone()
    }

    ///////////////////// WRITE ACCESSORS //////////////////////////////

    pub fn set_space_alias(&self, id: &str, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.set_space(id, name);
    }

    pub fn remove_space_alias(&self, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.remove_space(name);
    }

    pub fn remove_spaces_alias(&self) {
        let mut inner = self.inner.write();
        inner.lookup.remove_spaces();
    }

    pub fn set_project_alias(&self, name: String, proj: ProjectLookup) -> Result<()> {
        let mut inner = self.inner.write();
        trace! {
            id = %proj.id,
            name = %name,
            route = ?proj.node_route,
            identity_id = ?proj.identity_id,
            "Project stored in lookup table"
        };
        inner.lookup.set_project(name, proj);
        Ok(())
    }

    pub fn remove_project_alias(&self, name: &str) {
        let mut inner = self.inner.write();
        inner.lookup.remove_project(name);
    }

    pub fn remove_projects_alias(&self) {
        let mut inner = self.inner.write();
        inner.lookup.remove_projects();
    }
}
