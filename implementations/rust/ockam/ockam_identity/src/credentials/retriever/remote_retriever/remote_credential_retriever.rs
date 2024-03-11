use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_node::compat::asynchronous::RwLock;
use tracing::debug;

use ockam_core::{Address, AllowAll, DenyAll, Mailboxes, Result};
use ockam_node::Context;

use crate::models::CredentialAndPurposeKey;
use crate::{
    CredentialIssuer, CredentialRefresher, CredentialRetriever, CredentialsCache, Identifier,
    RemoteCredentialRefresherTimingOptions, RemoteCredentialRetrieverInfo,
    RemoteCredentialRetrieverTimingOptions, SecureChannels,
};

/// Credentials retriever for credentials located on a different node, issued by a specific authority.
/// The credentials are cached locally and can be periodically refreshed.
#[derive(Clone)]
pub struct RemoteCredentialRetriever {
    ctx: Arc<Context>,
    /// This issuer can issue credentials for a given subject
    credential_issuer: Arc<CredentialIssuer>,
    /// This cache store the retrieved credentials and checks that they are not expired when retrieved from storage
    credentials_cache: Arc<CredentialsCache>,
    /// These options are used to create credential refreshers
    refresher_timing_options: RemoteCredentialRefresherTimingOptions,
    /// List of credential refreshers. Each refresher refreshes the credential of a given identity
    /// by calling the remote_credential_retriever above (i.e. against the same authority)
    refreshers: Arc<RwLock<BTreeMap<Identifier, Arc<CredentialRefresher>>>>,
}

impl RemoteCredentialRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        ctx: Arc<Context>,
        secure_channels: Arc<SecureChannels>,
        issuer_info: RemoteCredentialRetrieverInfo,
        retriever_timing_options: RemoteCredentialRetrieverTimingOptions,
        refresher_timing_options: RemoteCredentialRefresherTimingOptions,
    ) -> Self {
        debug!(
            "Creation of RemoteCredentialRetriever for authority: {}",
            issuer_info.issuer
        );

        let remote_cached_credential_retriever = Arc::new(CredentialIssuer::new(
            ctx.clone(),
            secure_channels.clone(),
            issuer_info.clone(),
            retriever_timing_options,
        ));

        let cached_credential_retriever = Arc::new(CredentialsCache::new(
            secure_channels.identities().cached_credentials_repository(),
        ));

        Self {
            ctx,
            credential_issuer: remote_cached_credential_retriever,
            credentials_cache: cached_credential_retriever,
            refresher_timing_options,
            refreshers: Default::default(),
        }
    }
}

#[async_trait]
impl CredentialRetriever for RemoteCredentialRetriever {
    async fn retrieve(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        self.retrieve_credential(subject).await
    }
}

impl RemoteCredentialRetriever {
    /// Retrieve the credential for a given identifier
    async fn retrieve_credential(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        self.credential_issuer.get_credential_for(subject).await
    }

    /// Return a struct which will refresh the subject credential in the background.
    /// The CredentialRefresher interface allows workers to subscribe to refreshed credentials
    pub async fn make_refresher(&self, subject: &Identifier) -> Result<Arc<CredentialRefresher>> {
        debug!(
            "Requested RemoteCredentialRefresher for: {} and authority {}",
            subject,
            self.issuer()
        );
        let refreshers = self.refreshers.read().await;
        if let Some(existing_refresher) = refreshers.get(subject) {
            debug!(
                "Returning existing RemoteCredentialRefresher for: {} and authority {}",
                subject,
                self.issuer()
            );
            return Ok(existing_refresher.clone());
        }
        drop(refreshers);

        let mut refreshers = self.refreshers.write().await;
        if let Some(existing_refresher) = refreshers.get(subject) {
            debug!(
                "Returning existing RemoteCredentialRefresher for: {} and authority {}",
                subject,
                self.issuer()
            );
            return Ok(existing_refresher.clone());
        }

        debug!(
            "Creating new RemoteCredentialRefresher for: {}, authority: {}",
            subject,
            self.credential_issuer.issuer()
        );

        let mailboxes = Mailboxes::main(
            Address::random_tagged("RemoteCredentialRefresher"),
            Arc::new(DenyAll),
            Arc::new(AllowAll),
        );
        let ctx = self.ctx.new_detached_with_mailboxes(mailboxes).await?;
        let refresher = Arc::new(CredentialRefresher::new(
            Arc::new(ctx),
            self.credential_issuer.clone(),
            self.credentials_cache.clone(),
            subject.clone(),
            self.refresher_timing_options,
        ));
        refresher.initialize().await?;

        refreshers.insert(subject.clone(), refresher.clone());
        Ok(refresher)
    }

    /// Identifier of the credential issuer
    fn issuer(&self) -> &Identifier {
        self.credential_issuer.issuer()
    }
}
