use std::path::PathBuf;

use ockam::identity::models::ChangeHistory;
use serde::{Deserialize, Serialize};

use ockam::identity::Identifier;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::{Display, Formatter};

use crate::authenticator::PreTrustedIdentities;
use crate::config::lookup::InternetAddress;
use crate::nodes::service::default_address::DefaultAddress;

/// Configuration for the Authority node
#[derive(Debug, Clone)]
pub struct Configuration {
    /// Authority identity or identity associated with the newly created node
    pub identifier: Identifier,

    /// path where the database should be stored
    pub database_path: PathBuf,

    /// Project id on the Orchestrator node
    pub project_identifier: String,

    /// listener address for the TCP listener, for example "127.0.0.1:4000"
    pub tcp_listener_address: InternetAddress,

    /// service name for the secure channel listener, for example "api"
    /// The default is DefaultAddress::SECURE_CHANNEL_LISTENER
    pub secure_channel_listener_name: Option<String>,

    /// Service name for the direct authenticator, for example "direct_authenticator"
    /// The default is DefaultAddress::DIRECT_AUTHENTICATOR
    pub authenticator_name: Option<String>,

    /// list of trusted identities (identities with the ockam-role: enroller)
    pub trusted_identities: PreTrustedIdentities,

    /// If true don't start the direct authenticator service
    pub no_direct_authentication: bool,

    /// If true don't start the token enroller service
    pub no_token_enrollment: bool,

    /// optional configuration for the okta service
    pub okta: Option<OktaConfiguration>,

    /// Account Authority identity
    pub account_authority: Option<ChangeHistory>,

    /// Differentiate between admins and enrollers
    pub enforce_admin_checks: bool,

    /// Will not include trust_context_id and project id into credential
    /// Set to true after old clients are updated
    pub disable_trust_context_id: bool,
}

/// Local and private functions for the authority configuration
impl Configuration {
    /// Return the authority identity identifier
    pub(crate) fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    /// Return the project id as bytes
    pub(crate) fn project_identifier(&self) -> String {
        self.project_identifier.clone()
    }

    /// Return the address for the TCP listener
    pub(crate) fn tcp_listener_address(&self) -> InternetAddress {
        self.tcp_listener_address.clone()
    }

    /// Return the service name for the secure_channel_listener
    pub(crate) fn secure_channel_listener_name(&self) -> String {
        self.secure_channel_listener_name
            .clone()
            .unwrap_or(DefaultAddress::SECURE_CHANNEL_LISTENER.into())
    }

    /// Return the service name for the direct authenticator
    pub(crate) fn authenticator_name(&self) -> String {
        self.authenticator_name
            .clone()
            .unwrap_or(DefaultAddress::DIRECT_AUTHENTICATOR.to_string())
    }
}

/// Configuration for the Okta service
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OktaConfiguration {
    pub address: String,
    pub tenant_base_url: String,
    pub certificate: String,

    /// list of attribute names managed by Okta
    pub attributes: Vec<String>,
}

impl OktaConfiguration {
    /// Return the tenant base URL as str
    pub(crate) fn tenant_base_url(&self) -> &str {
        self.tenant_base_url.as_str()
    }

    /// Return the certificate name as str
    pub(crate) fn certificate(&self) -> &str {
        self.certificate.as_str()
    }

    /// Return the list of attributes managed by Okta as a vector of str
    pub(crate) fn attributes(&self) -> Vec<String> {
        self.attributes.clone()
    }
}

/// This struct represents an identity that the Authority accepts
/// as having all its attributes fully authenticated
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TrustedIdentity {
    identifier: Identifier,
    attributes: HashMap<String, String>,
}

impl Display for TrustedIdentity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| fmt::Error)?
                .as_str(),
        )
    }
}

impl TrustedIdentity {
    pub fn new(identifier: &Identifier, attributes: &HashMap<String, String>) -> TrustedIdentity {
        TrustedIdentity {
            identifier: identifier.clone(),
            attributes: attributes.clone(),
        }
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }
}
