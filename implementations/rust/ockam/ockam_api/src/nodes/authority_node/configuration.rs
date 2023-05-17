use crate::bootstrapped_identities_store::PreTrustedIdentities;
use crate::DefaultAddress;
use ockam::identity::credential::Timestamp;
use ockam::identity::{AttributesEntry, IdentityIdentifier};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Configuration for the Authority node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    /// Authority identity or identity associated with the newly created node
    pub identifier: IdentityIdentifier,

    /// path where the storage for identity attributes should be persisted
    pub storage_path: PathBuf,

    /// path where secrets should be persisted
    pub vault_path: PathBuf,

    /// Project identifier on the Orchestrator node
    pub project_identifier: String,

    /// Trust context identifier on the Orchestrator node
    pub trust_context_identifier: String,

    /// listener address for the TCP listener, for example "127.0.0.1:4000"
    pub tcp_listener_address: String,

    /// service name for the secure channel listener, for example "secure"
    /// The default is DefaultAddress::SECURE_CHANNEL_LISTENER
    pub secure_channel_listener_name: Option<String>,

    /// Service name for the direct authenticator, for example "api"
    /// The default is DefaultAddress::SECURE_CHANNEL_LISTENER
    pub authenticator_name: Option<String>,

    /// list of trusted identities (identities with the ockam-role: enroller)
    pub trusted_identities: PreTrustedIdentities,

    /// If true don't start the direct authenticator service
    pub no_direct_authentication: bool,

    /// If true don't start the token enroller service
    pub no_token_enrollment: bool,

    /// optional configuration for the okta service
    pub okta: Option<OktaConfiguration>,
}

/// Local and private functions for the authority configuration
impl Configuration {
    /// Return the authority identity identifier
    pub(crate) fn identifier(&self) -> IdentityIdentifier {
        self.identifier.clone()
    }

    /// Return the project identifier as bytes
    pub(crate) fn project_identifier(&self) -> String {
        self.project_identifier.clone()
    }

    /// Return the trust context identifier as bytes
    pub(crate) fn trust_context_identifier(&self) -> String {
        self.trust_context_identifier.clone()
    }

    /// Return the address for the TCP listener
    pub(crate) fn tcp_listener_address(&self) -> String {
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
    pub(crate) fn attributes(&self) -> Vec<&str> {
        self.attributes.iter().map(|a| a.as_str()).collect()
    }
}

/// This struct represents an identity that the Authority accepts
/// as having all its attributes fully authenticated
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TrustedIdentity {
    identifier: IdentityIdentifier,
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
    pub fn new(
        identifier: &IdentityIdentifier,
        attributes: &HashMap<String, String>,
    ) -> TrustedIdentity {
        TrustedIdentity {
            identifier: identifier.clone(),
            attributes: attributes.clone(),
        }
    }

    pub fn identifier(&self) -> IdentityIdentifier {
        self.identifier.clone()
    }

    pub fn attributes_entry(
        &self,
        project_identifier: String,
        authority_identifier: &IdentityIdentifier,
    ) -> AttributesEntry {
        let mut map: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (name, value) in self.attributes.clone().iter() {
            map.insert(name.clone(), value.as_bytes().to_vec());
        }

        // Since the authority node is started for a given project
        // add the project_id attribute to the trusted identities
        map.insert(
            "project_id".to_string(), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
            project_identifier.as_bytes().to_vec(),
        );

        map.insert(
            "trust_context_id".to_string(),
            project_identifier.as_bytes().to_vec(),
        );
        AttributesEntry::new(
            map,
            Timestamp::now().unwrap(),
            None,
            Some(authority_identifier.clone()),
        )
    }
}
