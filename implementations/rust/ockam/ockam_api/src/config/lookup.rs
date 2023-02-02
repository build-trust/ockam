use crate::cli_state::{CliStateError, Result};
use crate::cloud::project::{OktaAuth0, Project};
use crate::config::atomic::AtomicUpdater;
use crate::error::ApiError;
use anyhow::Context as _;
use bytes::Bytes;
use ockam_core::compat::collections::VecDeque;
use ockam_core::CowStr;
use ockam_identity::{IdentityIdentifier, PublicIdentity};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{
    collections::BTreeMap,
    fmt,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

#[derive(Clone, Debug)]
pub struct LookupConfigHandle {
    path: PathBuf,
    inner: Arc<RwLock<LookupConfig>>,
}

impl TryFrom<&PathBuf> for LookupConfigHandle {
    type Error = CliStateError;

    fn try_from(path: &PathBuf) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            path: path.clone(),
            inner: Arc::new(RwLock::new(LookupConfig::try_from(path)?)),
        })
    }
}

impl Default for LookupConfigHandle {
    fn default() -> Self {
        Self {
            path: PathBuf::from(""),
            inner: Arc::new(RwLock::new(LookupConfig::default())),
        }
    }
}

impl LookupConfigHandle {
    pub fn new(dir: PathBuf) -> Result<Self> {
        let path = dir.join("lookup.json");
        let inner = if !path.exists() {
            let inner = LookupConfig::default();
            let contents = serde_json::to_string(&inner)?;
            std::fs::write(&path, contents)?;
            inner
        } else {
            LookupConfig::try_from(&path)?
        };
        Ok(Self {
            path,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    fn persist(&self) -> Result<()> {
        AtomicUpdater::new(self.path.clone(), self.inner.clone()).run()
    }

    pub fn inner(&self) -> LookupConfig {
        self.inner.read().unwrap().clone()
    }

    pub fn set_space(&self, id: &str, name: &str) -> Result<()> {
        {
            let mut lock = self.inner.write().unwrap();
            lock.map.insert(
                format!("/space/{}", name),
                LookupValue::Space(SpaceLookup { id: id.to_string() }),
            );
        }
        self.persist()
    }

    pub fn get_space(&self, name: &str) -> Option<SpaceLookup> {
        let lock = self.inner.read().unwrap();
        let space = lock
            .map
            .get(&format!("/space/{}", name))
            .and_then(|value| match value {
                LookupValue::Space(space) => Some(space.clone()),
                _ => None,
            });
        space
    }

    pub fn remove_space(&self, name: &str) -> Result<Option<SpaceLookup>> {
        let space = {
            let mut lock = self.inner.write().unwrap();
            lock.map
                .remove(&format!("/space/{}", name))
                .and_then(|value| match value {
                    LookupValue::Space(v) => Some(v),
                    _ => None,
                })
        };
        self.persist()?;
        Ok(space)
    }

    pub fn remove_spaces(&self) -> Result<()> {
        {
            let mut lock = self.inner.write().unwrap();
            lock.map.retain(|k, _| !k.starts_with("/space/"));
        }
        self.persist()
    }

    /// Store a project route and identifier as lookup
    pub fn set_project(&self, name: String, proj: ProjectLookup) -> Result<()> {
        {
            let mut lock = self.inner.write().unwrap();
            lock.map
                .insert(format!("/project/{}", name), LookupValue::Project(proj));
        }
        self.persist()
    }

    pub fn get_project(&self, name: &str) -> Option<ProjectLookup> {
        let lock = self.inner.read().unwrap();
        let project = lock
            .map
            .get(&format!("/project/{}", name))
            .and_then(|value| match value {
                LookupValue::Project(project) => Some(project.clone()),
                _ => None,
            });
        project
    }

    pub fn remove_project(&self, name: &str) -> Result<Option<ProjectLookup>> {
        let project = {
            let mut lock = self.inner.write().unwrap();
            lock.map
                .remove(&format!("/project/{}", name))
                .and_then(|value| match value {
                    LookupValue::Project(v) => Some(v),
                    _ => None,
                })
        };
        self.persist()?;
        Ok(project)
    }

    pub fn remove_projects(&self) -> Result<()> {
        {
            let mut lock = self.inner.write().unwrap();
            lock.map.retain(|k, _| !k.starts_with("/project/"));
        }
        self.persist()
    }

    pub fn has_unresolved_projects(&self, meta: &LookupMeta) -> bool {
        for name in &meta.project {
            if self.get_project(name).is_none() {
                return true;
            }
        }
        false
    }

    pub fn projects(&self) -> BTreeMap<String, ProjectLookup> {
        let lock = self.inner.read().unwrap();
        lock.map
            .iter()
            .filter_map(|(k, v)| {
                if let LookupValue::Project(p) = v {
                    let name = k.strip_prefix("/project/").unwrap_or(k).to_string();
                    Some((name, p.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// A generic lookup mechanism for configuration values
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LookupConfig {
    #[serde(flatten)]
    map: BTreeMap<String, LookupValue>,
}

impl TryFrom<&PathBuf> for LookupConfig {
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

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LookupValue {
    Address(InternetAddress),
    Space(SpaceLookup),
    Project(ProjectLookup),
}

impl PartialEq for LookupValue {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_string(self).expect("Failed to serialize LookupValue")
            == serde_json::to_string(other).expect("Failed to serialize LookupValue")
    }
}

impl Eq for LookupValue {}

/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InternetAddress {
    /// DNSaddr and port
    Dns(String, u16),
    /// An IPv4 socket address
    V4(SocketAddrV4),
    /// An IPv6 socket address
    V6(SocketAddrV6),
}

impl Default for InternetAddress {
    fn default() -> Self {
        InternetAddress::Dns("localhost".to_string(), 6252)
    }
}

impl fmt::Display for InternetAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            match self {
                Self::Dns(addr, port) => format!("{}:{}", addr, port),
                Self::V4(v4) => format!("{}", v4),
                Self::V6(v6) => format!("{}", v6),
            }
            .as_str(),
        )
    }
}

impl InternetAddress {
    pub fn new(addr: &str) -> Option<Self> {
        // We try to parse a SocketAddress first, and if this fails
        // then assume it's a DNS address
        match SocketAddr::from_str(addr) {
            Ok(addr) => match addr {
                SocketAddr::V4(v4) => Some(Self::V4(v4)),
                SocketAddr::V6(v6) => Some(Self::V6(v6)),
            },
            Err(_) => {
                let addr_parts: Vec<&str> = addr.split(':').collect();
                if addr_parts.len() != 2 {
                    return None;
                }

                Some(Self::Dns(
                    addr_parts[0].to_string(),
                    addr_parts[1].parse().ok()?,
                ))
            }
        }
    }

    pub fn from_dns(s: String, port: u16) -> Self {
        Self::Dns(s, port)
    }

    /// Get the port for this address
    pub fn port(&self) -> u16 {
        match self {
            Self::Dns(_, port) => *port,
            Self::V4(v4) => v4.port(),
            Self::V6(v6) => v6.port(),
        }
    }
}

impl From<SocketAddr> for InternetAddress {
    fn from(sa: SocketAddr) -> Self {
        match sa {
            SocketAddr::V4(v4) => Self::V4(v4),
            SocketAddr::V6(v6) => Self::V6(v6),
        }
    }
}

/// Represents a remote Ockam space lookup
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpaceLookup {
    /// Identifier of this space
    pub id: String,
}

/// Represents a remote Ockam project lookup
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProjectLookup {
    /// How to reach the node hosting this project
    pub node_route: Option<MultiAddr>,
    /// Identifier of this project
    pub id: String,
    /// Identifier of the IDENTITY of the project (for secure-channel)
    pub identity_id: Option<IdentityIdentifier>,
    /// Project authority information.
    pub authority: Option<ProjectAuthority>,
    /// OktaAuth0 information.
    pub okta: Option<OktaAuth0>,
}

#[cfg(test)]
impl PartialEq for ProjectLookup {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_string(&self)
            .unwrap()
            .eq(&serde_json::to_string(&other).unwrap())
    }
}

impl ProjectLookup {
    pub async fn from_project(project: &Project<'_>) -> anyhow::Result<Self> {
        let node_route: MultiAddr = project
            .access_route
            .as_ref()
            .try_into()
            .context("Invalid project node route")?;
        let pid = project
            .identity
            .as_ref()
            .context("Project should have identity set")?;
        let authority = ProjectAuthority::from_raw(
            &project.authority_access_route,
            &project.authority_identity,
        )
        .await?;
        let okta = project.okta_config.as_ref().map(|o| OktaAuth0 {
            tenant_base_url: o.tenant_base_url.to_string(),
            client_id: o.client_id.to_string(),
            certificate: o.certificate.to_string(),
        });

        Ok(ProjectLookup {
            node_route: Some(node_route),
            id: project.id.to_string(),
            identity_id: Some(pid.clone()),
            authority,
            okta,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectAuthority {
    id: IdentityIdentifier,
    address: MultiAddr,
    identity: Bytes,
}

impl ProjectAuthority {
    pub fn new(id: IdentityIdentifier, addr: MultiAddr, identity: Vec<u8>) -> Self {
        Self {
            id,
            address: addr,
            identity: identity.into(),
        }
    }

    pub async fn from_raw<'a>(
        route: &'a Option<CowStr<'a>>,
        identity: &'a Option<CowStr<'a>>,
    ) -> Result<Option<Self>> {
        if let Some(r) = route {
            let rte = MultiAddr::try_from(&**r)?;
            let a = identity
                .as_ref()
                .ok_or_else(|| ApiError::generic("Identity is not set"))?;
            let a =
                hex::decode(&**a).map_err(|_| ApiError::generic("Invalid project authority"))?;
            let v = Vault::default();
            let p = PublicIdentity::import(&a, &v).await?;
            Ok(Some(ProjectAuthority::new(p.identifier().clone(), rte, a)))
        } else {
            Ok(None)
        }
    }

    pub fn identity(&self) -> &[u8] {
        &self.identity
    }

    pub fn identity_id(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
}

#[derive(Debug, Default)]
pub struct LookupMeta {
    /// Append any project name that is encountered during look-up
    pub project: VecDeque<Name>,
}

pub type Name = String;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn handle_spaces_lookups_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();
        let expected = SpaceLookup {
            id: "id1".to_string(),
        };

        l.set_space("id1", "space1").unwrap();
        assert_eq!(&l.get_space("space1").unwrap(), &expected);
        assert!(l.get_space("space2").is_none());
        assert_eq!(&l.remove_space("space1").unwrap().unwrap(), &expected);
        assert!(l.remove_space("space2").unwrap().is_none());
    }

    #[test]
    fn handle_spaces_lookups_from_disk() {
        let dir = tempfile::tempdir().unwrap();

        let add_some_entries = |dir: &TempDir| {
            let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();
            l.remove_spaces().unwrap();
            l.set_space("id1", "space1").unwrap();
            l.set_space("id2", "space2").unwrap();
            l.set_space("id3", "space3").unwrap();
        };

        let check_from_disk = |dir: &TempDir| {
            let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();
            let expected = [
                (
                    "space1",
                    SpaceLookup {
                        id: "id1".to_string(),
                    },
                ),
                (
                    "space2",
                    SpaceLookup {
                        id: "id2".to_string(),
                    },
                ),
                (
                    "space3",
                    SpaceLookup {
                        id: "id3".to_string(),
                    },
                ),
            ];
            for (name, space) in expected.iter() {
                assert_eq!(&l.get_space(name).unwrap(), space);
            }
            l.remove_spaces().unwrap();
            for (name, _) in expected.iter() {
                assert!(l.get_space(name).is_none());
            }
        };

        // Add some entries and check that they can be loaded from disk
        add_some_entries(&dir);
        check_from_disk(&dir);
    }

    #[test]
    fn handle_projects_lookups_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();
        let expected = ProjectLookup {
            id: "id1".to_string(),
            ..Default::default()
        };

        l.set_project("project1".to_string(), expected.clone())
            .unwrap();
        assert!(&l.get_project("project1").unwrap().eq(&expected));
        assert!(l.get_project("project2").is_none());
        assert!(&l.remove_project("project1").unwrap().unwrap().eq(&expected));
        assert!(l.remove_project("project2").unwrap().is_none());
    }

    #[test]
    fn handle_projects_lookups_from_disk() {
        let dir = tempfile::tempdir().unwrap();

        let expected = [
            (
                "project1",
                ProjectLookup {
                    id: "id1".to_string(),
                    ..Default::default()
                },
            ),
            (
                "project2",
                ProjectLookup {
                    id: "id2".to_string(),
                    ..Default::default()
                },
            ),
            (
                "project3",
                ProjectLookup {
                    id: "id3".to_string(),
                    ..Default::default()
                },
            ),
        ];

        let add_some_entries = |dir: &TempDir| {
            let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();
            l.remove_projects().unwrap();
            for (name, project) in expected.iter() {
                l.set_project(name.to_string(), project.clone()).unwrap();
            }
        };

        let check_from_disk = |dir: &TempDir| {
            let l = LookupConfigHandle::new(dir.path().to_path_buf()).unwrap();

            for (name, project) in expected.iter() {
                assert!(&l.get_project(name).unwrap().eq(project));
            }
            l.remove_projects().unwrap();
            for (name, _) in expected.iter() {
                assert!(l.get_project(name).is_none());
            }
        };

        // Add some entries and check that they can be loaded from disk
        add_some_entries(&dir);
        check_from_disk(&dir);
    }
}
