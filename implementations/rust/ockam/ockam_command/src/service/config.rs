use std::path::Path;

use miette::{Context as _, IntoDiagnostic};
use serde::{Deserialize, Serialize};

use crate::Result;
use ockam::identity::Identifier;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::StartInfluxDBLeaseManagerRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
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
    pub(crate) influxdb_token_lessor: Option<StartInfluxDBLeaseManagerRequest>,
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

fn sec_listener_default_addr() -> String {
    DefaultAddress::SECURE_CHANNEL_LISTENER.to_string()
}
