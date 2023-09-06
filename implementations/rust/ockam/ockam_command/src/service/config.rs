use std::path::Path;

use miette::{Context as _, IntoDiagnostic};
use serde::{Deserialize, Serialize};

use ockam::identity::Identifier;
use ockam_api::DefaultAddress;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatorConfig {
    #[serde(default = "authenticator_default_addr")]
    pub(crate) address: String,

    pub(crate) project: String,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OktaIdentityProviderConfig {
    #[serde(default = "okta_identity_provider_default_addr")]
    pub(crate) address: String,

    pub(crate) tenant_base_url: String,

    pub(crate) certificate: String,

    pub(crate) project: String,

    pub(crate) attributes: Vec<String>,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfigs {
    pub(crate) secure_channel_listener: Option<SecureChannelListenerConfig>,
    pub(crate) authenticator: Option<AuthenticatorConfig>,
    pub(crate) okta_identity_provider: Option<OktaIdentityProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn sec_listener_default_addr() -> String {
    DefaultAddress::SECURE_CHANNEL_LISTENER.to_string()
}

fn authenticator_default_addr() -> String {
    DefaultAddress::DIRECT_AUTHENTICATOR.to_string()
}

fn okta_identity_provider_default_addr() -> String {
    DefaultAddress::OKTA_IDENTITY_PROVIDER.to_string()
}
