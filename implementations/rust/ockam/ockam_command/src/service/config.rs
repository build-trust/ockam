use anyhow::{anyhow, Context, Result};
use ockam::identity::IdentityIdentifier;
use ockam_api::DefaultAddress;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    #[serde(default = "vault_default_addr")]
    pub(crate) address: String,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    #[serde(default = "identity_default_addr")]
    pub(crate) address: String,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureChannelListenerConfig {
    #[serde(default = "sec_listener_default_addr")]
    pub(crate) address: String,

    #[serde(default)]
    pub(crate) authorized_identifiers: Option<Vec<IdentityIdentifier>>,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    #[serde(default = "verifier_default_addr")]
    pub(crate) address: String,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatorConfig {
    #[serde(default = "authenticator_default_addr")]
    pub(crate) address: String,

    pub(crate) enrollers: PathBuf,

    pub(crate) project: String,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OktaIdentityProviderConfig {
    #[serde(default = "okta_identity_provider_default_addr")]
    pub(crate) address: String,

    pub(crate) tenant: String,

    pub(crate) certificate: String,

    pub(crate) project: String,

    pub(crate) attributes: Vec<String>,

    #[serde(default)]
    pub(crate) disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfigs {
    pub(crate) vault: Option<VaultConfig>,
    pub(crate) identity: Option<IdentityConfig>,
    pub(crate) secure_channel_listener: Option<SecureChannelListenerConfig>,
    pub(crate) verifier: Option<VerifierConfig>,
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
            .with_context(|| anyhow!("failed to read {:?}", path.as_ref()))?;
        serde_json::from_str(&s).with_context(|| anyhow!("invalid config {:?}", path.as_ref()))
    }
}

fn vault_default_addr() -> String {
    DefaultAddress::VAULT_SERVICE.to_string()
}

fn identity_default_addr() -> String {
    DefaultAddress::IDENTITY_SERVICE.to_string()
}

fn sec_listener_default_addr() -> String {
    DefaultAddress::SECURE_CHANNEL_LISTENER.to_string()
}

fn verifier_default_addr() -> String {
    DefaultAddress::VERIFIER.to_string()
}

fn authenticator_default_addr() -> String {
    DefaultAddress::AUTHENTICATOR.to_string()
}

fn okta_identity_provider_default_addr() -> String {
    DefaultAddress::OKTA_IDENTITY_PROVIDER.to_string()
}
