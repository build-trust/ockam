use miette::{Context as _, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

use crate::Result;
use ockam::identity::Identifier;
use ockam_abac::PolicyExpression::BooleanExpression;
use ockam_abac::{BooleanExpr, PolicyExpression};
use ockam_api::nodes::service::default_address::DefaultAddress;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub(crate) start_default_services: bool,
    pub(crate) startup_services: Option<ServiceConfigs>,
}

impl Config {
    pub(crate) fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(path.as_ref())
            .into_diagnostic()
            .context(format!("failed to read {:?}", path.as_ref()))?;
        let c = serde_json::from_str(&s)
            .into_diagnostic()
            .context(format!("invalid config {:?}", path.as_ref()))?;
        Ok(c)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceConfigs {
    pub(crate) secure_channel_listener: Option<SecureChannelListenerConfig>,
    pub(crate) control_api: Option<ControlApiConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecureChannelListenerConfig {
    #[serde(default = "sec_listener_default_addr")]
    pub(crate) address: String,

    #[serde(default)]
    pub(crate) authorized_identifiers: Option<Vec<Identifier>>,

    #[serde(default)]
    pub(crate) disabled: bool,

    #[serde(default)]
    pub(crate) identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ControlApiNodeResolution {
    #[default]
    Relay,
    DirectConnection,
}

fn default_control_api_bind_address() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 4080))
}

fn default_frontend_policy() -> PolicyExpression {
    BooleanExpression(BooleanExpr::from_str("node_control_api_frontend").unwrap())
}

fn default_backend_policy() -> PolicyExpression {
    BooleanExpression(BooleanExpr::from_str("node_control_api_backend").unwrap())
}

fn default_node_port() -> u16 {
    4100
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlApiConfig {
    #[serde(default)]
    pub(crate) frontend: bool,

    #[serde(default)]
    pub(crate) backend: bool,

    #[serde(default = "default_frontend_policy")]
    pub(crate) frontend_policy: PolicyExpression,

    #[serde(default = "default_backend_policy")]
    pub(crate) backend_policy: PolicyExpression,

    /// How to reach nodes.
    #[serde(default)]
    pub(crate) node_resolution: ControlApiNodeResolution,

    #[serde(default = "default_control_api_bind_address")]
    pub(crate) http_bind_address: SocketAddr,

    /// Port to use when connecting to nodes.
    #[serde(default = "default_node_port")]
    pub(crate) node_port: u16,

    /// Suffix to use when connecting to nodes.
    #[serde(default)]
    pub(crate) node_resolution_suffix: String,

    /// Authentication token for the control API.
    /// When undefined, the environment variable `OCKAM_CONTROL_API_AUTHENTICATION_TOKEN` will be used.
    pub(crate) authentication_token: Option<String>,
}

fn sec_listener_default_addr() -> String {
    DefaultAddress::SECURE_CHANNEL_LISTENER.to_string()
}
