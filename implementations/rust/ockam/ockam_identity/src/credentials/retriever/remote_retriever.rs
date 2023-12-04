use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use tracing::{info, trace};

use ockam_core::api::Request;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, Result, Route};
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::{Context, DEFAULT_TIMEOUT};
use ockam_transport_core::Transport;

use crate::models::CredentialAndPurposeKey;
use crate::{
    CachedCredentialRetriever, CredentialRetriever, Identifier, SecureChannels, SecureClient,
};

/// Credentials retriever for credentials located on a different node
pub struct RemoteCredentialRetriever {
    transport: Arc<dyn Transport>,
    secure_channels: Arc<SecureChannels>,
    info: RemoteCredentialRetrieverInfo,
    retrieving_credential: Mutex<()>,
}

impl RemoteCredentialRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        transport: Arc<dyn Transport>,
        secure_channels: Arc<SecureChannels>,
        info: RemoteCredentialRetrieverInfo,
    ) -> Self {
        Self {
            transport,
            secure_channels,
            info,
            retrieving_credential: Default::default(),
        }
    }

    async fn request_new_credential(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        debug!("Getting a new credential from: {}", &self.info.route);

        let cache = self
            .secure_channels
            .identities
            .cached_credentials_repository();

        let client = SecureClient::new(
            self.secure_channels.clone(),
            None,
            self.transport.clone(),
            self.info.route.clone(),
            &self.info.issuer,
            for_identity,
            DEFAULT_TIMEOUT,
        );

        let credential_result = client
            .ask(ctx, "credential_issuer", Request::post("/"))
            .await?
            .success();

        let credential = match credential_result {
            Ok(credential) => {
                info!(
                    "Retrieved a new credential for {} from {}",
                    for_identity, &self.info.route
                );

                credential
            }
            Err(err) => {
                error!(
                    "Getting credential from: {} failed with err: {}",
                    &self.info.route, err
                );
                return Err(err);
            }
        };

        let credential_data = self
            .secure_channels
            .identities()
            .credentials()
            .credentials_verification()
            .verify_credential(Some(for_identity), &[self.info.issuer.clone()], &credential)
            .await?;

        trace!("The retrieved credential is valid");

        let caching_res = cache
            .put(
                for_identity,
                &self.info.issuer,
                credential_data.credential_data.expires_at,
                credential.clone(),
            )
            .await;

        if let Some(err) = caching_res.err() {
            error!(
                "Error caching credential for {} from {}. Err={}",
                for_identity, &self.info.issuer, err
            );
        }

        Ok(credential)
    }
}

#[async_trait]
impl CredentialRetriever for RemoteCredentialRetriever {
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        debug!(
            "Requested credential for: {} from: {}",
            for_identity, self.info.issuer
        );

        let cache = self
            .secure_channels
            .identities
            .cached_credentials_repository();

        let cached_credential = CachedCredentialRetriever::retrieve_impl(
            &self.info.issuer,
            for_identity,
            cache.clone(),
        )
        .await?;
        if let Some(cached_credential) = cached_credential {
            return Ok(Some(cached_credential));
        }

        let _lock = self.retrieving_credential.lock().await;

        // Recheck in case other thread retrieved a credential
        debug!(
            "Trying again to get a cached credential for: {} from: {}",
            for_identity, self.info.issuer
        );
        let cached_credential =
            CachedCredentialRetriever::retrieve_impl(&self.info.issuer, for_identity, cache)
                .await?;
        if let Some(cached_credential) = cached_credential {
            return Ok(Some(cached_credential));
        }

        let credential = self.request_new_credential(ctx, for_identity).await;

        Ok(Some(credential?))
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub issuer: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(issuer: Identifier, route: Route, service_address: Address) -> Self {
        Self {
            issuer,
            route,
            service_address,
        }
    }
}
