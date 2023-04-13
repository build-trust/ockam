//! Configuration files used by the ockam CLI

use crate::cli_state::{CliStateError, CredentialState, StateItemTrait};
use crate::cloud::project::Project;
use crate::config::{lookup::ConfigLookup, ConfigValues};
use crate::error::ApiError;
use crate::{cli_state, multiaddr_to_transport_route, DefaultAddress, HexByteVec};
use ockam_core::compat::sync::Arc;
use ockam_core::{Result, Route};
use ockam_identity::credential::Credential;
use ockam_identity::{
    identities, AuthorityService, CredentialsMemoryRetriever, CredentialsRetriever, Identities,
    Identity, IdentityIdentifier, RemoteCredentialsRetriever, RemoteCredentialsRetrieverInfo,
    SecureChannels, TrustContext,
};
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::TcpTransport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use super::lookup::ProjectLookup;

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
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct TrustContextConfig {
    id: String,
    authority: Option<TrustAuthorityConfig>,
    path: Option<PathBuf>,
}

impl TrustContextConfig {
    pub fn new(id: String, authority: Option<TrustAuthorityConfig>) -> Self {
        Self {
            id,
            authority,
            path: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn authority(&self) -> Result<&TrustAuthorityConfig> {
        self.authority
            .as_ref()
            .ok_or_else(|| ApiError::generic("Missing authority on trust context config"))
    }

    pub async fn to_trust_context(
        &self,
        secure_channels: Arc<SecureChannels>,
        tcp_transport: Option<TcpTransport>,
    ) -> Result<TrustContext> {
        let authority = if let Some(authority_config) = self.authority.as_ref() {
            let identity = authority_config.identity().await?;
            let credential_retriever =
                if let Some(retriever_type) = &authority_config.own_credential {
                    Some(
                        retriever_type
                            .to_credential_retriever(secure_channels.clone(), tcp_transport)
                            .await?,
                    )
                } else {
                    None
                };

            Some(AuthorityService::new(
                secure_channels.identities().identities_reader(),
                secure_channels.identities().credentials(),
                identity.identifier(),
                credential_retriever,
            ))
        } else {
            None
        };

        Ok(TrustContext::new(self.id.clone(), authority))
    }

    pub fn from_authority_identity(
        authority_identity: &str,
        credential: Option<CredentialState>,
    ) -> Result<Self> {
        let own_cred = credential.map(CredentialRetrieverConfig::FromPath);
        let trust_context = TrustContextConfig::new(
            authority_identity.to_string(),
            Some(TrustAuthorityConfig::new(
                authority_identity.to_string(),
                own_cred,
            )),
        );

        Ok(trust_context)
    }
}

impl TryFrom<CredentialState> for TrustContextConfig {
    type Error = CliStateError;

    fn try_from(state: CredentialState) -> std::result::Result<Self, Self::Error> {
        let issuer = state.config().issuer.clone();
        let identity = issuer.export_hex()?;
        let retriever = CredentialRetrieverConfig::FromPath(state);
        let authority = TrustAuthorityConfig::new(identity, Some(retriever));
        Ok(TrustContextConfig::new(
            issuer.identifier().to_string(),
            Some(authority),
        ))
    }
}

impl TryFrom<Project> for TrustContextConfig {
    type Error = CliStateError;

    fn try_from(project_info: Project) -> std::result::Result<TrustContextConfig, Self::Error> {
        let authority = match (
            &project_info.authority_access_route,
            &project_info.authority_identity,
        ) {
            (Some(route), Some(identity)) => {
                let authority_route = MultiAddr::from_str(route)
                    .map_err(|_| ApiError::generic("incorrect multi address"))?;
                let retriever = CredentialRetrieverConfig::FromCredentialIssuer(
                    CredentialIssuerConfig::new(identity.to_string(), authority_route),
                );
                let authority = TrustAuthorityConfig::new(identity.to_string(), Some(retriever));
                Some(authority)
            }
            _ => None,
        };

        Ok(TrustContextConfig::new(project_info.id, authority))
    }
}

impl TryFrom<ProjectLookup> for TrustContextConfig {
    type Error = ApiError;

    fn try_from(
        project_lookup: ProjectLookup,
    ) -> std::result::Result<TrustContextConfig, ApiError> {
        let proj_auth = project_lookup
            .authority
            .as_ref()
            .expect("Project lookup is missing authority");
        let public_identity = hex::encode(proj_auth.identity());
        let authority = {
            let retriever = CredentialRetrieverConfig::FromCredentialIssuer(
                CredentialIssuerConfig::new(public_identity.clone(), proj_auth.address().clone()),
            );
            let authority = TrustAuthorityConfig::new(public_identity, Some(retriever));
            Some(authority)
        };

        Ok(TrustContextConfig::new(
            project_lookup.id.clone(),
            authority,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustAuthorityConfig {
    identity: String,
    own_credential: Option<CredentialRetrieverConfig>,
}

impl TrustAuthorityConfig {
    pub fn new(identity: String, own_credential: Option<CredentialRetrieverConfig>) -> Self {
        Self {
            identity,
            own_credential,
        }
    }

    pub fn identity_str(&self) -> &str {
        &self.identity
    }

    pub async fn identity(&self) -> Result<Identity> {
        identities()
            .identities_creation()
            .decode_identity(
                &hex::decode(&self.identity)
                    .map_err(|_| ApiError::generic("unable to decode authority identity"))?,
            )
            .await
    }

    pub fn own_credential(&self) -> Result<&CredentialRetrieverConfig> {
        self.own_credential
            .as_ref()
            .ok_or_else(|| ApiError::generic("Missing own credential on trust authority config"))
    }
}

/// Type of credential retriever
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum CredentialRetrieverConfig {
    /// Credential is stored in memory
    FromMemory(Credential),
    /// Path to credential file
    FromPath(CredentialState),
    /// MultiAddr to Credential Issuer
    FromCredentialIssuer(CredentialIssuerConfig),
}

impl CredentialRetrieverConfig {
    async fn to_credential_retriever(
        &self,
        secure_channels: Arc<SecureChannels>,
        tcp_transport: Option<TcpTransport>,
    ) -> Result<Arc<dyn CredentialsRetriever>> {
        match self {
            CredentialRetrieverConfig::FromMemory(credential) => Ok(Arc::new(
                CredentialsMemoryRetriever::new(credential.clone()),
            )),
            CredentialRetrieverConfig::FromPath(state) => Ok(Arc::new(
                CredentialsMemoryRetriever::new(state.config().credential()?),
            )),
            CredentialRetrieverConfig::FromCredentialIssuer(issuer_config) => {
                let _ = tcp_transport.ok_or_else(|| ApiError::generic("TCP Transport was not provided when credential retriever was defined as an issuer."))?;
                let credential_issuer_info = RemoteCredentialsRetrieverInfo::new(
                    issuer_config.resolve_identity().await?.identifier(),
                    issuer_config.resolve_route().await?,
                    DefaultAddress::CREDENTIAL_ISSUER.into(),
                );

                Ok(Arc::new(RemoteCredentialsRetriever::new(
                    secure_channels,
                    credential_issuer_info,
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

    pub async fn to_identities(&self, identities: Arc<Identities>) -> Result<Vec<Identity>> {
        let mut v = Vec::new();
        for a in self.authorities.values() {
            v.push(
                identities
                    .identities_creation()
                    .decode_identity(a.identity.as_slice())
                    .await?,
            )
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialIssuerConfig {
    pub identity: String,
    pub multiaddr: MultiAddr,
}

impl CredentialIssuerConfig {
    pub fn new(encoded_identity: String, multiaddr: MultiAddr) -> CredentialIssuerConfig {
        CredentialIssuerConfig {
            identity: encoded_identity,
            multiaddr,
        }
    }

    async fn resolve_route(&self) -> Result<Route> {
        let Some(route) = multiaddr_to_transport_route(&self.multiaddr) else {
            let err_msg = format!("Invalid route within trust context: {}", &self.multiaddr);
            error!("{err_msg}");
            return Err(ApiError::generic(&err_msg));
        };
        Ok(route)
    }

    async fn resolve_identity(&self) -> Result<Identity> {
        let encoded = hex::decode(&self.identity)
            .map_err(|_| ApiError::generic("Invalid project authority"))?;
        identities()
            .identities_creation()
            .decode_identity(&encoded)
            .await
    }
}
