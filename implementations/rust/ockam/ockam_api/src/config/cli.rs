//! Configuration files used by the ockam CLI

use crate::cli_state::CredentialState;
use crate::config::{lookup::ConfigLookup, ConfigValues};
use crate::error::ApiError;
use crate::{
    cli_state, CredentialIssuerInfo, CredentialIssuerRetriever, CredentialStateRetriever,
    HexByteVec,
};
use ockam::TcpTransport;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_identity::credential::Credential;
use ockam_identity::{
    AuthorityInfo, CredentialMemoryRetriever, CredentialRetriever, IdentityIdentifier,
    IdentityVault, PublicIdentity, TrustContext,
};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The main ockam CLI configuration
///
/// Used to determine CLI runtime behaviour and index existing nodes
/// on a system.
///
/// ## Updates
///
/// This configuration is read and updated by the user-facing `ockam`
/// CLI.  Furthermore the data is only relevant for user-facing
/// `ockam` CLI instances.  As such writes to this config don't have
/// to be synchronised to detached consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OckamConfig {
    /// We keep track of the project directories at runtime but don't
    /// persist this data to the configuration
    #[serde(skip)]
    pub dir: Option<PathBuf>,
    #[serde(default = "default_lookup")]
    pub lookup: ConfigLookup,
}

fn default_lookup() -> ConfigLookup {
    ConfigLookup::default()
}

impl ConfigValues for OckamConfig {
    fn default_values() -> Self {
        Self {
            dir: Some(Self::dir()),
            lookup: default_lookup(),
        }
    }
}

impl OckamConfig {
    /// Determine the default storage location for the ockam config
    pub fn dir() -> PathBuf {
        cli_state::CliState::default_dir().unwrap()
    }

    /// This function could be zero-copy if we kept the lock on the
    /// backing store for as long as we needed it.  Because this may
    /// have unwanted side-effects, instead we eagerly copy data here.
    /// This may be optimised in the future!
    pub fn lookup(&self) -> &ConfigLookup {
        &self.lookup
    }
}

/// A configuration struct to serialize and deserialize a trust context
/// used within the ockam CLI and ockam node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustContextConfig {
    id: String,
    authority: Option<TrustAuthorityConfig>,
}

impl TrustContextConfig {
    pub fn new(id: String, authority: Option<TrustAuthorityConfig>) -> Self {
        Self { id, authority }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn authority(&self) -> Option<&TrustAuthorityConfig> {
        self.authority.as_ref()
    }

    pub async fn into_trust_context(
        &self,
        tcp_transport: Option<TcpTransport>,
    ) -> Result<TrustContext> {
        let authority = if let Some(authority_config) = self.authority.as_ref() {
            let identity = authority_config.identity.clone();
            let own_cred = if let Some(retriever_type) = &authority_config.own_credential {
                Some(
                    retriever_type
                        .into_credential_retriever(tcp_transport)
                        .await?,
                )
            } else {
                None
            };

            Some(AuthorityInfo::new(identity, own_cred))
        } else {
            None
        };

        Ok(TrustContext::new(self.id.clone(), authority))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAuthorityConfig {
    identity: PublicIdentity,
    own_credential: Option<CredentialRetrieverType>,
}

impl TrustAuthorityConfig {
    pub fn new(identity: PublicIdentity, own_credential: Option<CredentialRetrieverType>) -> Self {
        Self {
            identity,
            own_credential,
        }
    }
}

/// Type of credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialRetrieverType {
    /// Credential is stored in memory
    FromMemory(Credential),
    /// Path to credential file
    FromPath(CredentialState),
    /// MultiAddr to Credential Issuer
    FromCredentialIssuer(CredentialIssuerInfo),
}

impl CredentialRetrieverType {
    async fn into_credential_retriever(
        &self,
        tcp_transport: Option<TcpTransport>,
    ) -> Result<Arc<dyn CredentialRetriever>> {
        match self {
            CredentialRetrieverType::FromMemory(credential) => {
                Ok(Arc::new(CredentialMemoryRetriever::new(credential.clone())))
            }
            CredentialRetrieverType::FromPath(credential_state) => Ok(Arc::new(
                CredentialStateRetriever::new(credential_state.clone()),
            )),
            CredentialRetrieverType::FromCredentialIssuer(credential_issuer_info) => {
                let tcp_transport = tcp_transport.ok_or_else(|| ApiError::generic("TCP Transport was not provided when credential retriever was defined as an issuer."))?;
                Ok(Arc::new(CredentialIssuerRetriever::new(
                    credential_issuer_info.clone(),
                    tcp_transport,
                )))
            }
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuthoritiesConfig {
    authorities: BTreeMap<IdentityIdentifier, Authority>,
}

impl AuthoritiesConfig {
    pub fn add_authority(&mut self, i: IdentityIdentifier, a: Authority) {
        self.authorities.insert(i, a);
    }

    pub fn authorities(&self) -> impl Iterator<Item = (&IdentityIdentifier, &Authority)> {
        self.authorities.iter()
    }

    pub async fn to_public_identities(
        &self,
        vault: Arc<dyn IdentityVault>,
    ) -> Result<Vec<PublicIdentity>> {
        let mut v = Vec::new();
        for a in self.authorities.values() {
            v.push(PublicIdentity::import(a.identity.as_slice(), vault.clone()).await?)
        }
        Ok(v)
    }
}

impl ConfigValues for AuthoritiesConfig {
    fn default_values() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Authority {
    identity: HexByteVec,
    access: MultiAddr,
}

impl Authority {
    pub fn new(identity: Vec<u8>, addr: MultiAddr) -> Self {
        Self {
            identity: identity.into(),
            access: addr,
        }
    }

    pub fn identity(&self) -> &[u8] {
        self.identity.as_slice()
    }

    pub fn access_route(&self) -> &MultiAddr {
        &self.access
    }
}
