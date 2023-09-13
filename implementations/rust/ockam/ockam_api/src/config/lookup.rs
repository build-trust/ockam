use crate::cli_state::{ProjectState, StateItemTrait};
use crate::cloud::project::{OktaAuth0, Project};
use crate::error::ApiError;
use bytes::Bytes;
use miette::WrapErr;
use ockam::identity::{identities, Identifier};
use ockam_core::compat::collections::VecDeque;
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

#[derive(Debug, Default)]
pub struct LookupMeta {
    /// Append any project name that is encountered during look-up
    pub project: VecDeque<Name>,
}

pub type Name = String;

/// A generic lookup mechanism for configuration values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigLookup {
    #[serde(flatten)]
    pub map: BTreeMap<String, LookupValue>,
}

impl Default for ConfigLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLookup {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn spaces(&self) -> impl Iterator<Item = (String, SpaceLookup)> + '_ {
        self.map.iter().filter_map(|(k, v)| {
            if let LookupValue::Space(p) = v {
                let name = k.strip_prefix("/space/").unwrap_or(k).to_string();
                Some((name, p.clone()))
            } else {
                None
            }
        })
    }

    pub fn set_space(&mut self, id: &str, name: &str) {
        self.map.insert(
            format!("/space/{name}"),
            LookupValue::Space(SpaceLookup { id: id.to_string() }),
        );
    }

    pub fn get_space(&self, name: &str) -> Option<&SpaceLookup> {
        self.map
            .get(&format!("/space/{name}"))
            .and_then(|value| match value {
                LookupValue::Space(space) => Some(space),
                _ => None,
            })
    }

    pub fn remove_space(&mut self, name: &str) -> Option<LookupValue> {
        self.map.remove(&format!("/space/{name}"))
    }

    pub fn remove_spaces(&mut self) {
        self.map.retain(|k, _| !k.starts_with("/space/"));
    }

    /// Store a project route and identifier as lookup
    pub fn set_project(&mut self, name: String, proj: ProjectLookup) {
        self.map
            .insert(format!("/project/{name}"), LookupValue::Project(proj));
    }

    pub fn get_project(&self, name: &str) -> Option<&ProjectLookup> {
        self.map
            .get(&format!("/project/{name}"))
            .and_then(|value| match value {
                LookupValue::Project(project) => Some(project),
                _ => None,
            })
    }

    pub fn remove_project(&mut self, name: &str) -> Option<LookupValue> {
        self.map.remove(&format!("/project/{name}"))
    }

    pub fn remove_projects(&mut self) {
        self.map.retain(|k, _| !k.starts_with("/project/"));
    }

    pub fn has_unresolved_projects(&self, meta: &LookupMeta) -> bool {
        meta.project
            .iter()
            .any(|name| self.get_project(name).is_none())
    }

    pub fn projects(&self) -> impl Iterator<Item = (String, ProjectLookup)> + '_ {
        self.map.iter().filter_map(|(k, v)| {
            if let LookupValue::Project(p) = v {
                let name = k.strip_prefix("/project/").unwrap_or(k).to_string();
                Some((name, p.clone()))
            } else {
                None
            }
        })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LookupValue {
    Address(InternetAddress),
    Space(SpaceLookup),
    Project(ProjectLookup),
}

/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
                Self::Dns(addr, port) => format!("{addr}:{port}"),
                Self::V4(v4) => format!("{v4}"),
                Self::V6(v6) => format!("{v6}"),
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceLookup {
    /// Identifier of this space
    pub id: String,
}

/// Represents a remote Ockam project lookup
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectLookup {
    /// How to reach the node hosting this project
    pub node_route: Option<MultiAddr>,
    /// Identifier of this project
    pub id: String,
    /// Name of this project within
    pub name: String,
    /// Identifier of the IDENTITY of the project (for secure-channel)
    pub identity_id: Option<Identifier>,
    /// Project authority information.
    pub authority: Option<ProjectAuthority>,
    /// OktaAuth0 information.
    pub okta: Option<OktaAuth0>,
}

impl ProjectLookup {
    pub async fn from_project(project: &Project) -> ockam_core::Result<Self, ApiError> {
        let node_route: MultiAddr = project.access_route.as_str().try_into()?;
        let pid = project
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::message("Project should have identity set"))?;
        let authority = project.authority().await?;
        let okta = project.okta_config.as_ref().map(|o| OktaAuth0 {
            tenant_base_url: o.tenant_base_url.clone(),
            client_id: o.client_id.to_string(),
            certificate: o.certificate.to_string(),
        });

        Ok(ProjectLookup {
            node_route: Some(node_route),
            id: project.id.to_string(),
            name: project.name.to_string(),
            identity_id: Some(pid.clone()),
            authority,
            okta,
        })
    }

    pub async fn from_state(projects: Vec<ProjectState>) -> miette::Result<BTreeMap<String, Self>> {
        let mut lookups = BTreeMap::new();
        for p in projects {
            let l = ProjectLookup::from_project(p.config())
                .await
                .context("Failed to read project configuration")?;
            lookups.insert(l.name.clone(), l);
        }
        Ok(lookups)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProjectAuthority {
    id: Identifier,
    address: MultiAddr,
    identity: Bytes,
}

impl ProjectAuthority {
    pub fn new(id: Identifier, addr: MultiAddr, identity: Vec<u8>) -> Self {
        Self {
            id,
            address: addr,
            identity: identity.into(),
        }
    }

    pub async fn from_project(project: &Project) -> ockam_core::Result<Option<Self>, ApiError> {
        Self::from_raw(&project.authority_access_route, &project.authority_identity).await
    }

    pub async fn from_raw<S: ToString>(
        route: &Option<S>,
        identity: &Option<S>,
    ) -> ockam_core::Result<Option<Self>, ApiError> {
        if let Some(r) = route {
            let rte = MultiAddr::try_from(r.to_string().as_str())?;
            let a = identity
                .as_ref()
                .ok_or_else(|| ApiError::message("Identity is not set"))?
                .to_string();
            let a = hex::decode(a.as_str())
                .map_err(|_| ApiError::message("Invalid project authority"))?;
            let p = identities().identities_creation().import(None, &a).await?;
            Ok(Some(ProjectAuthority::new(p.identifier().clone(), rte, a)))
        } else {
            Ok(None)
        }
    }

    pub fn identity(&self) -> &[u8] {
        &self.identity
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.id
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
}
