use std::fmt::{Display, Formatter};
use std::sync::Arc;

use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey};
use ockam::identity::{
    AuthorityService, CredentialsMemoryRetriever, Identifier, Identity, RemoteCredentialsRetriever,
    RemoteCredentialsRetrieverInfo, SecureChannels, TrustContext,
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::TcpTransport;

use crate::cli_state::CliState;
use crate::multiaddr_to_transport_route;
use crate::nodes::service::default_address::DefaultAddress;

use super::Result;

impl CliState {
    pub async fn get_trust_context(&self, name: &str) -> Result<NamedTrustContext> {
        match self
            .trust_contexts_repository()
            .await?
            .get_trust_context(name)
            .await?
        {
            Some(trust_context) => Ok(trust_context),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no trust context with name {name}"),
            ))?,
        }
    }

    pub async fn get_default_trust_context(&self) -> Result<NamedTrustContext> {
        match self
            .trust_contexts_repository()
            .await?
            .get_default_trust_context()
            .await?
        {
            Some(trust_context) => Ok(trust_context),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default trust context",
            ))?,
        }
    }

    pub async fn get_trust_context_or_default(
        &self,
        name: &Option<String>,
    ) -> Result<NamedTrustContext> {
        match name {
            Some(name) => self.get_trust_context(name).await,
            None => self.get_default_trust_context().await,
        }
    }

    pub async fn retrieve_trust_context(
        &self,
        trust_context_name: &Option<String>,
        project_name: &Option<String>,
        authority_identity: &Option<Identity>,
        credential_name: &Option<String>,
    ) -> Result<Option<NamedTrustContext>> {
        match trust_context_name {
            Some(name) => Ok(Some(self.get_trust_context(name).await?)),
            None => {
                let project = match project_name {
                    Some(name) => self.get_project_by_name(name).await.ok(),
                    None => self.get_default_project().await.ok(),
                };
                match project {
                    Some(project) => Ok(self.get_trust_context(&project.name).await.ok()),
                    None => match credential_name {
                        Some(credential_name) => Ok(Some(
                            self.create_trust_context(
                                None,
                                None,
                                Some(credential_name.clone()),
                                authority_identity.clone(),
                                None,
                            )
                            .await?,
                        )),
                        None => Ok(None),
                    },
                }
            }
        }
    }

    pub async fn get_trust_contexts(&self) -> Result<Vec<NamedTrustContext>> {
        Ok(self
            .trust_contexts_repository()
            .await?
            .get_trust_contexts()
            .await?)
    }

    pub async fn delete_trust_context(&self, name: &str) -> Result<()> {
        Ok(self
            .trust_contexts_repository()
            .await?
            .delete_trust_context(name)
            .await?)
    }

    pub async fn set_default_trust_context(&self, name: &str) -> Result<()> {
        Ok(self
            .trust_contexts_repository()
            .await?
            .set_default_trust_context(name)
            .await?)
    }

    pub async fn create_trust_context(
        &self,
        name: Option<String>,
        trust_context_id: Option<String>,
        credential_name: Option<String>,
        authority_identity: Option<Identity>,
        authority_route: Option<MultiAddr>,
    ) -> Result<NamedTrustContext> {
        let credential = match credential_name {
            Some(name) => self.get_credential_by_name(&name).await.ok(),
            None => None,
        };
        let name = name.unwrap_or("default".to_string());

        // if the authority identity is not defined use the
        // authority identity defined on the credential
        let authority_identity = match authority_identity.clone() {
            Some(identity) => Some(identity),
            None => match credential.clone() {
                None => None,
                Some(credential) => Some(credential.issuer_identity().await?),
            },
        };

        let trust_context_id = trust_context_id
            .or_else(|| {
                authority_identity
                    .clone()
                    .map(|i| i.identifier().to_string())
            })
            .unwrap_or("default".to_string());

        let trust_context = NamedTrustContext::new(
            &name,
            &trust_context_id,
            credential.map(|c| c.credential_and_purpose_key()),
            authority_identity.map(|i| i.change_history().clone()),
            authority_route,
        );

        let repository = self.trust_contexts_repository().await?;
        repository.store_trust_context(&trust_context).await?;

        // If there is no previous default trust_context set this trust_context as the default
        let default_trust_context = repository.get_default_trust_context().await?;
        if default_trust_context.is_none() {
            repository
                .set_default_trust_context(&trust_context.name())
                .await?
        };

        Ok(trust_context)
    }
}

/// A NamedTrustContext collects all the data necessary to create a TrustContext
/// under a specific name:
///
/// Either we can
///   - retrieve a fixed credential
///   - access an authority node to retrieve credentials
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedTrustContext {
    name: String,
    trust_context_id: String,
    credential: Option<CredentialAndPurposeKey>,
    authority_change_history: Option<ChangeHistory>,
    authority_route: Option<MultiAddr>,
}

impl NamedTrustContext {
    pub fn new(
        name: &str,
        trust_context_id: &str,
        credential: Option<CredentialAndPurposeKey>,
        authority_identity: Option<ChangeHistory>,
        authority_route: Option<MultiAddr>,
    ) -> Self {
        Self {
            name: name.to_string(),
            trust_context_id: trust_context_id.to_string(),
            credential,
            authority_change_history: authority_identity,
            authority_route,
        }
    }
}

impl Display for NamedTrustContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name())?;
        Ok(())
    }
}

impl NamedTrustContext {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn trust_context_id(&self) -> String {
        self.trust_context_id.to_string()
    }

    pub fn credential(&self) -> Option<CredentialAndPurposeKey> {
        self.credential.clone()
    }

    /// Return the route to the trust context authority if configured
    pub fn authority_route(&self) -> Option<MultiAddr> {
        self.authority_route.clone()
    }

    /// Return the change history of the trust context authority if configured
    pub fn authority_change_history(&self) -> Option<ChangeHistory> {
        self.authority_change_history.clone()
    }

    /// Return the identity of the trust context authority if configured
    pub async fn authority_identity(&self) -> Result<Option<Identity>> {
        match &self.authority_change_history {
            Some(change_history) => Ok(Some(
                Identity::create_from_change_history(change_history).await?,
            )),
            None => Ok(None),
        }
    }

    /// Return the identifier of the trust context authority if configured
    pub async fn authority_identifier(&self) -> Result<Option<Identifier>> {
        Ok(self
            .authority_identity()
            .await?
            .map(|i| i.identifier().clone()))
    }

    /// Make a TrustContext
    /// This requires a transport and secure channels if we need to communicate with an Authority node
    pub async fn trust_context(
        &self,
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
    ) -> Result<TrustContext> {
        let authority_identifier = self.authority_identifier().await?;
        let authority_service = match (
            self.credential.clone(),
            authority_identifier,
            self.authority_route.clone(),
        ) {
            (Some(credential), Some(identifier), _) => {
                let credential_retriever = CredentialsMemoryRetriever::new(credential);
                Some(AuthorityService::new(
                    secure_channels.identities().credentials(),
                    identifier,
                    Some(Arc::new(credential_retriever)),
                ))
            }
            (None, Some(identifier), Some(route)) => {
                let credential_retriever = RemoteCredentialsRetriever::new(
                    Arc::new(tcp_transport.clone()),
                    secure_channels.clone(),
                    RemoteCredentialsRetrieverInfo::new(
                        identifier.clone(),
                        multiaddr_to_transport_route(&route).ok_or_else(|| {
                            Error::new(
                                Origin::Api,
                                Kind::Internal,
                                format!("cannot create a route from the address {route}"),
                            )
                        })?,
                        DefaultAddress::CREDENTIAL_ISSUER.into(),
                    ),
                );
                Some(AuthorityService::new(
                    secure_channels.identities().credentials(),
                    identifier,
                    Some(Arc::new(credential_retriever)),
                ))
            }
            (None, Some(identifier), None) => Some(AuthorityService::new(
                secure_channels.identities().credentials(),
                identifier.clone(),
                None,
            )),
            _ => None,
        };
        Ok(TrustContext::new(
            self.trust_context_id.clone(),
            authority_service,
        ))
    }

    /// Return access data for an authority in order to be able to create
    /// a RPC client to that authority and obtain credentials
    pub async fn authority(&self) -> Result<Option<Authority>> {
        match (
            self.authority_identifier().await?,
            self.authority_route.clone(),
        ) {
            (Some(identifier), Some(route)) => Ok(Some(Authority::new(identifier, route))),
            _ => Ok(None),
        }
    }
}

/// Configuration of an authority node
#[derive(Clone)]
pub struct Authority {
    identifier: Identifier,
    route: MultiAddr,
}

impl Authority {
    pub fn new(identifier: Identifier, route: MultiAddr) -> Self {
        Self { identifier, route }
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    pub fn route(&self) -> MultiAddr {
        self.route.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use ockam::identity::models::CredentialAndPurposeKey;
    use ockam::identity::models::CredentialSchemaIdentifier;
    use ockam::identity::utils::AttributesBuilder;
    use ockam::identity::{identities, Identifier, Identities};
    use ockam_core::env::FromString;

    use super::*;

    // There are 3 ways to create a trust context
    //  - with only an id
    //  - with a credential
    //  - with an authority identity + route
    #[tokio::test]
    async fn test_create_trust_context() -> Result<()> {
        let cli = CliState::test().await?;

        // 1. with only an id
        let result = cli
            .create_trust_context(
                Some("trust-context".into()),
                Some("1".into()),
                None,
                None,
                None,
            )
            .await?;
        let expected = NamedTrustContext::new("trust-context", "1", None, None, None);
        assert_eq!(result, expected);

        // that trust context is the default one because it is the first created trust context
        let result = cli.get_default_trust_context().await?;
        assert_eq!(result, expected);

        // 2. with a credential
        let identities = identities().await?;
        let authority_identifier = identities.identities_creation().create_identity().await?;
        let authority = identities.get_identity(&authority_identifier).await?;
        let credential = create_credential(identities, &authority_identifier).await?;
        cli.store_credential("credential-name", &authority, credential.clone())
            .await?;
        let result = cli
            .create_trust_context(
                Some("trust-context".into()),
                None,
                Some("credential-name".into()),
                None,
                None,
            )
            .await?;
        let expected = NamedTrustContext::new(
            "trust-context",
            authority.identifier().to_string().as_str(),
            Some(credential),
            Some(authority.change_history().clone()),
            None,
        );
        assert_eq!(result, expected);

        // 3. with an authority
        let authority_route = MultiAddr::from_string("/dnsaddr/127.0.0.1/tcp/5000/service/api")?;
        let result = cli
            .create_trust_context(
                Some("trust-context".into()),
                None,
                None,
                Some(authority.clone()),
                Some(authority_route.clone()),
            )
            .await?;
        let expected = NamedTrustContext::new(
            "trust-context",
            authority.identifier().to_string().as_str(),
            None,
            Some(authority.change_history().clone()),
            Some(authority_route),
        );
        assert_eq!(result, expected);
        Ok(())
    }

    /// HELPERS
    pub async fn create_credential(
        identities: Arc<Identities>,
        issuer: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        let subject = identities.identities_creation().create_identity().await?;

        let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
            .with_attribute("name".as_bytes().to_vec(), b"value".to_vec())
            .build();

        Ok(identities
            .credentials()
            .credentials_creation()
            .issue_credential(issuer, &subject, attributes, Duration::from_secs(1))
            .await?)
    }
}
