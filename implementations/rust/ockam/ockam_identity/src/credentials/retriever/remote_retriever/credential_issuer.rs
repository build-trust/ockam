use async_recursion::async_recursion;

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use tracing::debug;

use ockam_core::Result;
use ockam_node::Context;

use crate::models::CredentialAndPurposeKey;
use crate::{
    CredentialRequest, CredentialsCache, Identifier, IssuerClient, RemoteCredentialRetrieverInfo,
    RemoteCredentialRetrieverTimingOptions, SecureChannels,
};

/// This struct represents a credential issuer, which can issue credential for a given subject.
/// This issuer can be accessed via an IssuerClient (on a secure channel) and the issued credentials
/// are stored locally until they expire
#[derive(Clone)]
pub struct CredentialIssuer {
    /// Credential issuer
    issuer: Identifier,
    /// The secure channel registry is used to synchronize remote calls and make sure that
    /// we only try to retrieve the credentials for a given identity only once
    secure_channels: Arc<SecureChannels>,
    /// This client is used to get new credentials from a specific issuer
    issuer_client: Arc<IssuerClient>,
    /// This retriever is used to get local credentials if they are not expired
    /// and store new credentials retrieved with the remote credential retriever
    credentials_cache: Arc<CredentialsCache>,
}

impl CredentialIssuer {
    /// Create a new remote credential retriever
    pub fn new(
        ctx: Arc<Context>,
        secure_channels: Arc<SecureChannels>,
        issuer_info: RemoteCredentialRetrieverInfo,
        retriever_timing_options: RemoteCredentialRetrieverTimingOptions,
    ) -> Self {
        debug!(
            "Creation of RemoteCachedCredentialRetriever for authority: {}",
            issuer_info.issuer
        );

        let issuer_client = Arc::new(IssuerClient::new(
            ctx.clone(),
            secure_channels.clone(),
            issuer_info.clone(),
            retriever_timing_options,
        ));

        let credentials_cache = Arc::new(CredentialsCache::new(
            secure_channels.identities().cached_credentials_repository(),
        ));

        Self {
            issuer: issuer_info.issuer,
            secure_channels,
            issuer_client,
            credentials_cache,
        }
    }
}

impl CredentialIssuer {
    /// Get the credential for a given identifier by first checking if some valid credentials
    /// are available locally. If not, use the remote retriever and store the new credentials locally.
    pub async fn get_credential_for(
        &self,
        subject: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        if let Ok(credential_and_purpose_key) = self
            .credentials_cache
            .get_credential(self.issuer(), subject)
            .await
        {
            return Ok(credential_and_purpose_key);
        };

        self.renew_credential(subject).await
    }

    /// Call the issuer to ask for a new credential
    ///
    /// Note: this call is considered as recursive since it uses a client creating a secure channel
    /// to the issue
    #[async_recursion]
    pub async fn renew_credential(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        // make sure that only one request at the time is performed
        // by using the secure channel registry to store requests per pair issuer / subject
        self.secure_channels
            .secure_channel_registry()
            .get_credential_request_or(Arc::new(CredentialRequest::new(
                &self.issuer,
                subject,
                self.issuer_client.clone(),
                self.credentials_cache.clone(),
            )))
            .await?
            .run()
            .await
    }

    /// Identifier of the credential issuer
    pub fn issuer(&self) -> &Identifier {
        self.issuer_client.issuer()
    }
}
