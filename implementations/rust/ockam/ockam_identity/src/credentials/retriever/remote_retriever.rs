use ockam_core::api::Request;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing::trace;

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, Result, Route};
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::{Context, DEFAULT_TIMEOUT};

use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{CredentialsRetriever, Identifier, SecureChannels, SecureClient, TimestampInSeconds};

#[derive(Clone)]
struct CachedCredential {
    credential: CredentialAndPurposeKey,
    valid_until: TimestampInSeconds,
}

/// Credentials retriever for credentials located on a different node
pub struct RemoteCredentialsRetriever {
    secure_channels: Arc<SecureChannels>,
    inner_cache: Arc<Mutex<Option<CachedCredential>>>,
    issuer: RemoteCredentialsRetrieverInfo,
}

impl RemoteCredentialsRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        secure_channels: Arc<SecureChannels>,
        issuer: RemoteCredentialsRetrieverInfo,
    ) -> Self {
        Self {
            secure_channels,
            inner_cache: Default::default(),
            issuer,
        }
    }

    async fn make_secure_client(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<SecureClient> {
        let resolved_route = ctx
            .resolve_transport_route(self.issuer.route.clone())
            .await?;
        trace!(
            "Getting credential from resolved route: {}",
            resolved_route.clone()
        );

        Ok(SecureClient::new(
            self.secure_channels.clone(),
            resolved_route,
            &self.issuer.identifier,
            for_identity,
            DEFAULT_TIMEOUT,
        ))
    }
}

#[async_trait]
impl CredentialsRetriever for RemoteCredentialsRetriever {
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        debug!("Requested credential for: {}", for_identity);

        // check if we have a valid cached credential
        let mut guard = self.inner_cache.lock().await;
        let now = now()?;
        if let Some(cache) = guard.as_ref() {
            // add an extra minute to have a bit of leeway for clock skew
            if cache.valid_until > now + TimestampInSeconds(60) {
                debug!("Found valid cached credential for: {}", for_identity);
                return Ok(cache.credential.clone());
            }
        }

        debug!("Getting credential from: {}", &self.issuer.route);
        let client = self.make_secure_client(ctx, for_identity).await?;
        let credential = client
            .ask(ctx, "credential_issuer", Request::post("/"))
            .await?
            .success()?;

        debug!("Retrieved a credential for subject {}", for_identity);

        let credential_data = self
            .secure_channels
            .identities()
            .credentials()
            .credentials_verification()
            .verify_credential(
                Some(for_identity),
                &[self.issuer.identifier.clone()],
                &credential,
            )
            .await?;

        debug!("The retrieved credential is valid");

        *guard = Some(CachedCredential {
            credential: credential.clone(),
            valid_until: credential_data.credential_data.expires_at,
        });

        Ok(credential)
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialsRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub identifier: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialsRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(identifier: Identifier, route: Route, service_address: Address) -> Self {
        Self {
            identifier,
            route,
            service_address,
        }
    }
}
