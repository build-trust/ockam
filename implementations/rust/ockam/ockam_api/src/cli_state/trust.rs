use crate::cloud::project::Project;
use crate::nodes::service::{
    CredentialScope, NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions,
};
use crate::nodes::NodeManager;
use crate::{ApiError, CliState, TransportRouteResolver};
use ockam::identity::models::ChangeHistory;
use ockam::identity::{IdentitiesVerification, RemoteCredentialRetrieverInfo};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_multiaddr::MultiAddr;
use ockam_vault::SoftwareVaultForVerifyingSignatures;

impl CliState {
    async fn retrieve_trust_options_explicit_project_authority(
        &self,
        authority_identity: &str,
        authority_route: &Option<MultiAddr>,
        credential_scope: &Option<String>,
    ) -> Result<NodeManagerTrustOptions> {
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

        if let Some(authority_multiaddr) = authority_route {
            let scope = match credential_scope {
                Some(scope) => scope.clone(),
                None => {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::NotFound,
                        "Authority address was provided but credential scope was not provided",
                    ))
                }
            };

            let authority_route = TransportRouteResolver::default()
                .allow_tcp()
                .resolve(authority_multiaddr)
                .map_err(|err| {
                    Error::new(
                        Origin::Api,
                        Kind::NotFound,
                        format!("Invalid authority route. Err: {}", &err),
                    )
                })?;
            let info = RemoteCredentialRetrieverInfo::create_for_project_member(
                authority_identifier.clone(),
                authority_route,
            );

            let trust_options = NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::Remote { info, scope },
                NodeManagerCredentialRetrieverOptions::None,
                Some(authority_identifier.clone()),
                NodeManagerCredentialRetrieverOptions::None,
            );

            debug!(
                    "TrustOptions configured: Authority: {}. Credentials retrieved from Remote Authority: {}",
                    authority_identifier, authority_multiaddr
                );

            return Ok(trust_options);
        }

        if let Some(credential_scope) = credential_scope {
            let trust_options = NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::CacheOnly {
                    issuer: authority_identifier.clone(),
                    scope: credential_scope.clone(),
                },
                NodeManagerCredentialRetrieverOptions::None,
                Some(authority_identifier.clone()),
                NodeManagerCredentialRetrieverOptions::None,
            );

            debug!(
                "TrustOptions configured: Authority: {}. Expect credentials in cache",
                authority_identifier
            );

            return Ok(trust_options);
        }

        let trust_options = NodeManagerTrustOptions::new(
            NodeManagerCredentialRetrieverOptions::None,
            NodeManagerCredentialRetrieverOptions::None,
            Some(authority_identifier.clone()),
            NodeManagerCredentialRetrieverOptions::None,
        );

        debug!(
            "TrustOptions configured: Authority: {}. Only verifying credentials",
            authority_identifier
        );

        Ok(trust_options)
    }

    async fn retrieve_trust_options_with_project(
        &self,
        project: Project,
    ) -> Result<NodeManagerTrustOptions> {
        let authority_identifier = project
            .authority_identifier()
            .ok_or_else(|| ApiError::core("no authority identifier"))?;
        let authority_multiaddr = project.authority_multiaddr()?;
        let authority_route = TransportRouteResolver::default()
            .allow_tcp()
            .resolve(authority_multiaddr)
            .map_err(|err| {
                Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    format!("Invalid authority route. Err: {}", &err),
                )
            })?;

        let project_id = project.project_id().to_string();
        let project_member_retriever = NodeManagerCredentialRetrieverOptions::Remote {
            info: RemoteCredentialRetrieverInfo::create_for_project_member(
                authority_identifier.clone(),
                authority_route,
            ),
            scope: CredentialScope::ProjectMember {
                project_id: project_id.clone(),
            }
            .to_string(),
        };

        let controller_identifier = NodeManager::load_controller_identifier()?;
        let controller_transport_route = NodeManager::controller_route().await?;

        let project_admin_retriever = NodeManagerCredentialRetrieverOptions::Remote {
            info: RemoteCredentialRetrieverInfo::create_for_project_admin(
                controller_identifier.clone(),
                controller_transport_route.clone(),
                project_id.clone(),
            ),
            scope: CredentialScope::ProjectAdmin {
                project_id: project_id.clone(),
            }
            .to_string(),
        };

        let account_admin_retriever = NodeManagerCredentialRetrieverOptions::Remote {
            info: RemoteCredentialRetrieverInfo::create_for_account_admin(
                controller_identifier.clone(),
                controller_transport_route,
            ),
            scope: CredentialScope::AccountAdmin {
                // TODO: Should be account id, which is now known at this point, but it's not used
                //  yet anywhere
                account_id: project_id.clone(),
            }
            .to_string(),
        };

        let trust_options = NodeManagerTrustOptions::new(
            project_member_retriever,
            project_admin_retriever,
            Some(authority_identifier.clone()),
            account_admin_retriever,
        );

        debug!(
            "TrustOptions configured: Authority: {}. Credentials retrieved from project: {}",
            authority_identifier, authority_multiaddr
        );
        Ok(trust_options)
    }

    /// Create [`NodeManagerTrustOptions`] depending on what trust information we possess
    ///  1. Either we explicitly know the Authority identity that we trust, and optionally route to its node to request
    ///     a new credential
    ///  2. Or we know the project name (or have default one) that contains identity and route to the Authority node
    #[instrument(skip_all, fields(project_name = project_name.clone(), authority_identity = authority_identity.as_ref().map(|a| a.to_string()).unwrap_or("n/a".to_string()), authority_route = authority_route.clone().map_or("n/a".to_string(), |r| r.to_string())))]
    pub async fn retrieve_trust_options(
        &self,
        project_name: &Option<String>,
        authority_identity: &Option<ChangeHistory>,
        authority_route: &Option<MultiAddr>,
        credential_scope: &Option<String>,
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

        // We're using explicitly specified authority instead of a project
        if let Some(authority_identity) = authority_identity {
            return self
                .retrieve_trust_options_explicit_project_authority(
                    &authority_identity.export_as_string()?,
                    authority_route,
                    credential_scope,
                )
                .await;
        }

        let project = match project_name {
            Some(project_name) => self.projects().get_project_by_name(project_name).await.ok(),
            None => self.projects().get_default_project().await.ok(),
        };

        let project = match project {
            Some(project) => project,
            None => {
                debug!("TrustOptions configured: No Authority. No Credentials");
                return Ok(NodeManagerTrustOptions::new(
                    NodeManagerCredentialRetrieverOptions::None,
                    NodeManagerCredentialRetrieverOptions::None,
                    None,
                    NodeManagerCredentialRetrieverOptions::None,
                ));
            }
        };

        if project.authority_identifier().is_none() {
            debug!("TrustOptions configured: No Authority. No Credentials");
            return Ok(NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::None,
                NodeManagerCredentialRetrieverOptions::None,
                None,
                NodeManagerCredentialRetrieverOptions::None,
            ));
        }

        self.retrieve_trust_options_with_project(project).await
    }
}
