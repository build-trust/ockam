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

    /// Store a node identifier and address lookup
    pub fn set_node(&mut self, node: &str, address: InternetAddress) {
        self.map
            .insert(format!("/node/{}", node), LookupValue::Address(address));
    }

    pub fn get_node(&self, node: &str) -> Option<&InternetAddress> {
        self.map
            .get(&format!("/node/{}", node))
            .and_then(|value| match value {
                LookupValue::Address(addr) => Some(addr),
                _ => None,
            })
    }

    pub fn set_space(&mut self, id: &str, name: &str) {
        self.map.insert(
            format!("/space/{}", name),
            LookupValue::Space(SpaceLookup { id: id.to_string() }),
        );
    }

    pub fn get_space(&self, name: &str) -> Option<&SpaceLookup> {
        self.map
            .get(&format!("/space/{}", name))
            .and_then(|value| match value {
                LookupValue::Space(space) => Some(space),
                _ => None,
            })
    }

    pub fn remove_space(&mut self, name: &str) -> Option<LookupValue> {
        self.map.remove(&format!("/space/{}", name))
    }

    pub fn remove_spaces(&mut self) {
        self.map.retain(|k, _| !k.starts_with("/space/"));
    }

    /// Store a project route and identifier as lookup
    pub fn set_project(
        &mut self,
        name: String,
        node_route: String,
        id: String,
        identity_id: String,
    ) {
        self.map.insert(
            format!("/project/{}", name),
            LookupValue::Project(ProjectLookup {
                node_route,
                id,
                identity_id,
            }),
        );
    }

    pub fn get_project(&self, name: &str) -> Option<&ProjectLookup> {
        self.map
            .get(&format!("/project/{}", name))
            .and_then(|value| match value {
                LookupValue::Project(project) => Some(project),
                _ => None,
            })
    }

    pub fn remove_project(&mut self, name: &str) -> Option<LookupValue> {
        self.map.remove(&format!("/project/{}", name))
    }

    pub fn remove_projects(&mut self) {
        self.map.retain(|k, _| !k.starts_with("/project/"));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LookupValue {
    Address(InternetAddress),
    Space(SpaceLookup),
    Project(ProjectLookup),
}

/// An internet address abstraction (v6/v4/dns)
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceLookup {
    /// Identifier of this space
    pub id: String,
}

/// Represents a remote Ockam project lookup
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectLookup {
    /// How to reach the node hosting this project
    ///
    /// This value MUST be a MultiAddr and is checked before storing
    /// that it is.
    node_route: String,
    /// Identifier of this project
    pub id: String,
    /// Identifier of the IDENTITY of the project (for secure-channel)
    pub identity_id: String,
}

impl ProjectLookup {
    pub fn node_route(&self) -> MultiAddr {
        MultiAddr::from_str(&self.node_route).expect(
            "tried retrieving a MultiAddr from ProjectLookup where no MultiAddr had been stored",
        )
    }
}
