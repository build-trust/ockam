use crate::DefaultAddress;
use ockam_core::compat::collections::HashMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_identity::authenticated_storage::AttributesEntry;
use ockam_identity::credential::Timestamp;
use ockam_identity::IdentityIdentifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Configuration for the Authority node
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Configuration {
    /// path where the storage for identity attributes should be persisted
    pub storage_path: String,

    /// path where secrets should be persisted
    pub vault_path: String,

    /// Project identifier on the Orchestrator node
    pub project_identifier: String,

    /// listener address for the TCP listener, for example "127.0.0.1:4000"
    pub tcp_listener_address: String,

    /// service name for the secure channel listener, for example "secure"
    /// The default is DefaultAddress::SECURE_CHANNEL_LISTENER
    pub secure_channel_listener_name: Option<String>,

    /// Service name for the direct authenticator, for example "api"
    /// The default is DefaultAddress::SECURE_CHANNEL_LISTENER
    pub authenticator_name: Option<String>,

    /// list of trusted identities (identities with the ockam-role: enroller)
    pub trusted_identities: Vec<TrustedIdentity>,

    /// optional configuration for the okta service
    pub okta: Option<OktaConfiguration>,
}

/// Local and private functions for the authority configuration
impl Configuration {
    /// Return the project identifier as bytes
    pub(crate) fn project_identifier(&self) -> Vec<u8> {
        self.project_identifier.as_bytes().to_vec()
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

    /// Read the configuration either from a string or a file path
    pub fn read(path_or_string: &str) -> Result<Configuration> {
        Self::read_from_string(path_or_string).or_else(|_| Self::read_from_path(path_or_string))
    }

    /// Read the configuration from a file
    pub fn read_from_path(path: &str) -> Result<Configuration> {
        let path = PathBuf::from_str(path).unwrap();
        let contents =
            std::fs::read_to_string(path).map_err(|e| Error::new(Origin::Node, Kind::Io, e))?;
        Self::read_from_string(&contents)
    }

    /// Read the configuration from a JSON string
    pub fn read_from_string(contents: &str) -> Result<Configuration> {
        serde_json::from_str(contents).map_err(|e| Error::new(Origin::Node, Kind::Io, e))
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
            "project_id".to_string(),
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

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_read_configuration_as_json() {
        let actual = r#"
        {
          "vault_path": "/tmp/ockam/vault",
          "storage_path": "/tmp/ockam/storage",
          "project_identifier": "project",
          "tcp_listener_address": "127.0.0.1:4000",
          "secure_channel_listener_name": "secure",
          "authenticator_name": "direct_authenticator",
          "trusted_identities": [{
            "identifier": "Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638",
            "attributes": { "ockam-role": "enroller" }
          }],
          "okta": {
            "tenant_base_url": "okta.url",
            "certificate": "okta.ca",
            "address": "okta",
            "attributes": ["attribute_1", "attribute_2"]
          }
        }
        "#;
        let actual: Configuration = Configuration::read(actual).unwrap();

        let expected = Configuration {
            storage_path: "/tmp/ockam/storage".to_string(),
            vault_path: "/tmp/ockam/vault".to_string(),
            project_identifier: "project".to_string(),
            tcp_listener_address: "127.0.0.1:4000".to_string(),
            secure_channel_listener_name: Some("secure".to_string()),
            authenticator_name: Some("direct_authenticator".to_string()),
            trusted_identities: vec![TrustedIdentity {
                identifier: IdentityIdentifier::from_str(
                    "Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638",
                )
                .unwrap(),
                attributes: HashMap::from_iter(vec![(
                    "ockam-role".to_string(),
                    "enroller".to_string(),
                )]),
            }],
            okta: Some(OktaConfiguration {
                tenant_base_url: "okta.url".to_string(),
                certificate: "okta.ca".to_string(),
                address: "okta".to_string(),
                attributes: vec!["attribute_1".to_string(), "attribute_2".to_string()],
            }),
        };
        assert_eq!(actual, expected);
    }
}
