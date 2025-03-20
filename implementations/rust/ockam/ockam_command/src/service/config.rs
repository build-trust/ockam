use crate::util::parsers::duration_parser;
use crate::Result;
use miette::{miette, Context as _, IntoDiagnostic};
use ockam::identity::Identifier;
use ockam_abac::PolicyExpression::BooleanExpression;
use ockam_abac::{BooleanExpr, PolicyExpression};
use ockam_api::nodes::service::default_address::DefaultAddress;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServicesConfig {
    #[serde(
        alias = "startup_services",
        alias = "startup-services",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) services: Option<ServiceConfigs>,
}

impl ServicesConfig {
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(path.as_ref())
            .into_diagnostic()
            .context(format!(
                "failed to read services config from {:?}",
                path.as_ref()
            ))?;
        Self::from_string(&s)
    }

    pub(crate) fn from_string(contents: &str) -> Result<Self> {
        if let Ok(c) = serde_yaml::from_str(contents) {
            return Ok(c);
        }
        if let Ok(c) = serde_json::from_str(contents) {
            return Ok(c);
        }
        Err(miette!(format!("invalid config {:?}", contents)))
    }

    pub(crate) fn to_string(&self) -> Result<String> {
        serde_json::to_string(&self).into_diagnostic()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServiceConfigs {
    #[serde(
        alias = "secure-channel-listener",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) secure_channel_listener: Option<SecureChannelListenerConfig>,

    #[serde(alias = "control-api", skip_serializing_if = "Option::is_none")]
    pub(crate) control_api: Option<ControlApiConfig>,

    #[serde(
        alias = "portal-synchronization",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) portal_synchronization_config: Option<PortalSynchronizationConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SecureChannelListenerConfig {
    #[serde(default = "default_secure_listener_address")]
    pub(crate) address: String,

    #[serde(
        alias = "authorized-identifiers",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) authorized_identifiers: Option<Vec<Identifier>>,

    pub(crate) disabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
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

fn default_connection_node_port() -> u16 {
    4100
}

fn default_node_resolution_pattern() -> String {
    "{name}".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlApiConfig {
    #[serde(default)]
    pub(crate) frontend: bool,

    #[serde(default)]
    pub(crate) backend: bool,

    #[serde(alias = "frontend-policy", default = "default_frontend_policy")]
    pub(crate) frontend_policy: PolicyExpression,

    #[serde(alias = "backend-policy", default = "default_backend_policy")]
    pub(crate) backend_policy: PolicyExpression,

    /// How to reach nodes.
    #[serde(alias = "node-resolution", default)]
    pub(crate) node_resolution: ControlApiNodeResolution,

    #[serde(
        alias = "http-bind-address",
        default = "default_control_api_bind_address"
    )]
    pub(crate) http_bind_address: SocketAddr,

    /// Port to use when connecting to nodes.
    #[serde(
        alias = "connection-node-port",
        default = "default_connection_node_port"
    )]
    pub(crate) connection_node_port: u16,

    /// Pattern to use when connecting to nodes.
    /// {name} will be replaced with the node name.
    /// When `name` is "node1", and the pattern is "my-{name}.example.com", the resulting address
    /// will be "my-node1.example.com".
    #[serde(
        alias = "node-resolution-pattern",
        default = "default_node_resolution_pattern"
    )]
    pub(crate) node_resolution_pattern: String,

    /// During node resolution, the frontend expects the relays in the provided node.
    /// By default, relays are expected in the default project.
    /// To use local relay, use an empty string.
    #[serde(
        alias = "node-resolution-relay-node",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) node_resolution_relay_node: Option<String>,

    /// Authentication token for the control API.
    /// When undefined, the environment variable `OCKAM_CONTROL_API_AUTHENTICATION_TOKEN` will be used.
    #[serde(
        alias = "authentication-token",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) authentication_token: Option<String>,
}

fn default_secure_listener_address() -> String {
    DefaultAddress::SECURE_CHANNEL_LISTENER.to_string()
}

fn default_synchronization_interval() -> Duration {
    Duration::from_secs(10)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PortalSynchronizationConfig {
    #[serde(default)]
    pub(crate) enabled: bool,

    #[serde(default = "default_synchronization_interval", value_parser = duration_parser)]
    pub(crate) interval: Duration,
}
