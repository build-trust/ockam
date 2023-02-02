//! Configuration files used by the ockam CLI

use crate::cli_state::{CliStateError, Result};
use crate::config::atomic::AtomicUpdater;
use crate::HexByteVec;
use ockam_identity::{IdentityIdentifier, IdentityVault, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct AuthoritiesConfigHandle {
    path: PathBuf,
    inner: Arc<RwLock<AuthoritiesConfig>>,
}

impl TryFrom<&PathBuf> for AuthoritiesConfigHandle {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            path: path.clone(),
            inner: Arc::new(RwLock::new(AuthoritiesConfig::try_from(path)?)),
        })
    }
}

impl Default for AuthoritiesConfigHandle {
    fn default() -> Self {
        Self {
            path: PathBuf::from(""),
            inner: Arc::new(RwLock::new(AuthoritiesConfig::default())),
        }
    }
}

impl AuthoritiesConfigHandle {
    pub fn new(dir: PathBuf) -> Result<Self> {
        let path = dir.join("authorities.json");
        let inner = if !path.exists() {
            let inner = AuthoritiesConfig::default();
            let contents = serde_json::to_string(&inner)?;
            std::fs::write(&path, contents)?;
            inner
        } else {
            AuthoritiesConfig::try_from(&path)?
        };
        Ok(Self {
            path,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    fn persist(&self) -> Result<()> {
        AtomicUpdater::new(self.path.clone(), self.inner.clone()).run()
    }

    pub fn inner(&self) -> AuthoritiesConfig {
        self.inner.read().unwrap().clone()
    }

    pub fn add_authority(&self, i: IdentityIdentifier, a: Authority) -> Result<()> {
        {
            let mut lock = self.inner.write().unwrap();
            lock.authorities.insert(i, a);
        }
        self.persist()
    }

    pub fn authorities(&self) -> BTreeMap<IdentityIdentifier, Authority> {
        let lock = self.inner.read().unwrap();
        lock.authorities.clone()
    }

    pub async fn to_public_identities<V>(&self, vault: &V) -> Result<Vec<PublicIdentity>>
    where
        V: IdentityVault,
    {
        let authorities = {
            let lock = self.inner.read().unwrap();
            lock.authorities.clone()
        };
        let mut v = Vec::new();
        for a in authorities.values() {
            v.push(PublicIdentity::import(a.identity.as_slice(), vault).await?)
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuthoritiesConfig {
    pub authorities: BTreeMap<IdentityIdentifier, Authority>,
}

impl TryFrom<&PathBuf> for AuthoritiesConfig {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(serde_json::from_str(&contents)?),
            Err(_) => {
                let d = Self::default();
                let contents = serde_json::to_string(&d)?;
                std::fs::write(path, contents)?;
                Ok(d)
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Authority {
    identity: HexByteVec,
    access: MultiAddr,
}

impl Authority {
    pub fn new(identity: Vec<u8>, addr: MultiAddr) -> Self {
        Self {
            identity: identity.into(),
            access: addr,
        }
    }

    pub fn identity(&self) -> &[u8] {
        self.identity.as_slice()
    }

    pub fn access_route(&self) -> &MultiAddr {
        &self.access
    }
}
