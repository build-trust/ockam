use crate::models::CredentialAndPurposeKey;
use crate::{
    CredentialRetrieverOptions, Identifier, RemoteCredentialRetrieverInfo, SecureChannels,
    SecureClient,
};
use ockam_core::api::Request;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::Result;
use ockam_node::Context;
use tracing::{info, trace};

/// This client can be used to call an issuer to issue a credential for a given subject
#[derive(Clone)]
pub struct IssuerClient {
    ctx: Arc<Context>,
    secure_channels: Arc<SecureChannels>,
    issuer_info: RemoteCredentialRetrieverInfo,
    retriever_timing_options: RemoteCredentialRetrieverTimingOptions,
}

impl IssuerClient {
    /// Create a new client for getting credentials issued by a specific issuer
    pub fn new(
        ctx: Arc<Context>,
        secure_channels: Arc<SecureChannels>,
        issuer_info: RemoteCredentialRetrieverInfo,
        retriever_timing_options: RemoteCredentialRetrieverTimingOptions,
    ) -> Self {
        IssuerClient {
            ctx,
            secure_channels,
            issuer_info,
            retriever_timing_options,
        }
    }

    /// Retrieve credentials for a given identity by calling an issuer on a remote node
    pub async fn get_credential(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        let transport = self
            .ctx
            .get_registered_transport(self.issuer_info.transport_type)?;
        let client = SecureClient::new(
            self.secure_channels.clone(),
            CredentialRetrieverOptions::None,
            transport,
            self.issuer_info.route.clone(),
            &self.issuer_info.issuer,
            subject,
            self.retriever_timing_options
                .secure_channel_creation_timeout,
            self.retriever_timing_options.request_timeout,
        );

        let credential_and_purpose_key = client
            .ask(&self.ctx, "credential_issuer", Request::post("/"))
            .await?
            .success()?;

        info!(
            "Retrieved a new credential for {} from {}",
            subject, &self.issuer_info.route
        );

        let _ = self
            .secure_channels
            .identities()
            .credentials()
            .credentials_verification()
            .verify_credential(
                Some(subject),
                &[self.issuer().clone()],
                &credential_and_purpose_key,
            )
            .await?;
        trace!("The retrieved credential is valid");
        Ok(credential_and_purpose_key)
    }

    /// Identifier of the credential issuer
    pub fn issuer(&self) -> &Identifier {
        &self.issuer_info.issuer
    }
}

/// Default timeout for requesting credential from the authority
pub const DEFAULT_CREDENTIAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Default timeout for creating secure channel to the authority
pub const DEFAULT_CREDENTIAL_SECURE_CHANNEL_CREATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Timing options for retrieving remote credentials
#[derive(Debug, Clone, Copy)]
pub struct RemoteCredentialRetrieverTimingOptions {
    /// Timeout for request to the Authority node
    pub request_timeout: Duration,
    /// Timeout for creating secure channel to the Authority node
    pub secure_channel_creation_timeout: Duration,
}

impl Default for RemoteCredentialRetrieverTimingOptions {
    fn default() -> Self {
        Self {
            request_timeout: DEFAULT_CREDENTIAL_REQUEST_TIMEOUT,
            secure_channel_creation_timeout: DEFAULT_CREDENTIAL_SECURE_CHANNEL_CREATION_TIMEOUT,
        }
    }
}
