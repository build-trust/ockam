use crate::cloud::project::ProjectName;
use crate::nodes::service::{NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions};
use crate::{multiaddr_to_transport_route, CliState, DefaultAddress};
use ockam::identity::{IdentitiesVerification, RemoteCredentialRetrieverInfo};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_multiaddr::MultiAddr;
use ockam_vault::SoftwareVaultForVerifyingSignatures;

impl CliState {
    /// Create [`NodeManagerTrustOptions`] depending on what trust information we possess
    ///  1. Either we explicitly know the Authority identity that we trust, and optionally route to its node to request
    ///     a new credential
    ///  2. Or we know the project name (or have default one) that contains identity and route to the Authority node
    #[instrument(skip_all, fields(project_name = ?project_name, authority_identity = authority_identity.clone(), authority_route = authority_route.clone().map_or("n/a".to_string(), |r| r.to_string())))]
    pub async fn retrieve_trust_options(
        &self,
        project_name: &Option<ProjectName>,
        authority_identity: &Option<String>,
        authority_route: &Option<MultiAddr>,
        expect_cached_credential: bool,
    ) -> Result<NodeManagerTrustOptions> {
        if project_name.is_some() && (authority_identity.is_some() || authority_route.is_some()) {
            return Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "Both project_name and authority info are provided",
            ));
        }

        if authority_route.is_some() && authority_identity.is_none() {
            return Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "Authority address was provided but authority identity is unknown",
            ));
        }

        if authority_route.is_some() && expect_cached_credential {
            return Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "Authority address was provided but expect_cached_credential is true",
            ));
        }

        if let Some(authority_identity) = authority_identity {
            let identities_verification = IdentitiesVerification::new(
                self.change_history_repository(),
                SoftwareVaultForVerifyingSignatures::create(),
            );

            let authority_identity = hex::decode(authority_identity).map_err(|_e| {
                Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    "Invalid authority identity hex",
                )
            })?;
            let authority_identifier = identities_verification
                .import(None, &authority_identity)
                .await?;

            let trust_options = if let Some(authority_multiaddr) = authority_route {
                let authority_route =
                    multiaddr_to_transport_route(authority_multiaddr).ok_or(Error::new(
                        Origin::Api,
                        Kind::NotFound,
                        format!("Invalid authority route: {}", &authority_multiaddr),
                    ))?;
                let info = RemoteCredentialRetrieverInfo::new(
                    authority_identifier.clone(),
                    authority_route,
                    DefaultAddress::CREDENTIAL_ISSUER.into(),
                );

                let trust_options = NodeManagerTrustOptions::new(
                    NodeManagerCredentialRetrieverOptions::Remote(info),
                    Some(authority_identifier.clone()),
                );

                info!(
                    "TrustOptions configured: Authority: {}. Credentials retrieved from Remote Authority: {}",
                    authority_identifier, authority_multiaddr
                );

                trust_options
            } else if expect_cached_credential {
                let trust_options = NodeManagerTrustOptions::new(
                    NodeManagerCredentialRetrieverOptions::CacheOnly(authority_identifier.clone()),
                    Some(authority_identifier.clone()),
                );

                info!(
                    "TrustOptions configured: Authority: {}. Expect credentials in cache",
                    authority_identifier
                );

                trust_options
            } else {
                let trust_options = NodeManagerTrustOptions::new(
                    NodeManagerCredentialRetrieverOptions::None,
                    Some(authority_identifier.clone()),
                );

                info!(
                    "TrustOptions configured: Authority: {}. Only verifying credentials",
                    authority_identifier
                );

                trust_options
            };

            return Ok(trust_options);
        }

        let project = match project_name {
            Some(project_name) => self.get_project_by_name(project_name).await.ok(),
            None => self.get_default_project().await.ok(),
        };

        let project = match project {
            Some(project) => project,
            None => {
                info!("TrustOptions configured: No Authority. No Credentials");
                return Ok(NodeManagerTrustOptions::new(
                    NodeManagerCredentialRetrieverOptions::None,
                    None,
                ));
            }
        };

        let authority_identifier = project.authority_identifier().await?;
        let authority_multiaddr = project.authority_access_route()?;
        let authority_route =
            multiaddr_to_transport_route(&authority_multiaddr).ok_or(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("Invalid authority route: {}", &authority_multiaddr),
            ))?;
        let info = RemoteCredentialRetrieverInfo::new(
            authority_identifier.clone(),
            authority_route,
            DefaultAddress::CREDENTIAL_ISSUER.into(),
        );

        let trust_options = NodeManagerTrustOptions::new(
            NodeManagerCredentialRetrieverOptions::Remote(info),
            Some(authority_identifier.clone()),
        );

        info!(
            "TrustOptions configured: Authority: {}. Credentials retrieved from project: {}",
            authority_identifier, authority_multiaddr
        );
        Ok(trust_options)
    }
}
