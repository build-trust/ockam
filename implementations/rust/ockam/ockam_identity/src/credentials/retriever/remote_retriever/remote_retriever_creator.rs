use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, AllowAll, DenyAll, Mailboxes, Result};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_transport_core::Transport;
use tracing::debug;

use crate::{
    CredentialRetriever, CredentialRetrieverCreator, Identifier, RemoteCredentialRetriever,
    RemoteCredentialRetrieverInfo, RemoteCredentialRetrieverTimingOptions, SecureChannels,
};

/// Creator for [`RemoteCredentialRetriever`]
pub struct RemoteCredentialRetrieverCreator {
    ctx: Context,
    transport: Arc<dyn Transport>,
    secure_channels: Arc<SecureChannels>,
    info: RemoteCredentialRetrieverInfo,
    scope: String,
    timing_options: RemoteCredentialRetrieverTimingOptions,

    // Should be only one retriever per subject Identifier
    registry: RwLock<BTreeMap<Identifier, Arc<RemoteCredentialRetriever>>>,
}

impl RemoteCredentialRetrieverCreator {
    /// Constructor
    pub fn new(
        ctx: Context,
        transport: Arc<dyn Transport>,
        secure_channels: Arc<SecureChannels>,
        info: RemoteCredentialRetrieverInfo,
        scope: String,
    ) -> Self {
        Self {
            ctx,
            transport,
            secure_channels,
            info,
            scope,
            timing_options: Default::default(),
            registry: Default::default(),
        }
    }

    /// Constructor
    pub fn new_extended(
        ctx: Context,
        transport: Arc<dyn Transport>,
        secure_channels: Arc<SecureChannels>,
        info: RemoteCredentialRetrieverInfo,
        scope: String,
        timing_options: RemoteCredentialRetrieverTimingOptions,
    ) -> Self {
        Self {
            ctx,
            transport,
            secure_channels,
            info,
            scope,
            timing_options,
            registry: Default::default(),
        }
    }
}

#[async_trait]
impl CredentialRetrieverCreator for RemoteCredentialRetrieverCreator {
    async fn create(&self, subject: &Identifier) -> Result<Arc<dyn CredentialRetriever>> {
        debug!(
            "Requested RemoteCredentialRetriever for: {}, authority: {}",
            subject, self.info.issuer
        );

        let registry = self.registry.read().await;
        if let Some(existing_retriever) = registry.get(subject) {
            debug!(
                "Returning existing RemoteCredentialRetriever for: {}, authority: {}",
                subject, self.info.issuer
            );
            return Ok(existing_retriever.clone());
        }

        drop(registry);

        let mut registry = self.registry.write().await;
        if let Some(existing_retriever) = registry.get(subject) {
            debug!(
                "Returning existing RemoteCredentialRetriever for: {}, authority: {}",
                subject, self.info.issuer
            );
            return Ok(existing_retriever.clone());
        }

        debug!(
            "Creating new RemoteCredentialRetriever for: {}, authority: {}",
            subject, self.info.issuer
        );
        let mailboxes = Mailboxes::primary(
            Address::random_tagged("RemoteCredentialRetriever"),
            Arc::new(DenyAll),
            Arc::new(AllowAll),
        );
        let ctx = self.ctx.new_detached_with_mailboxes(mailboxes)?;
        let retriever = RemoteCredentialRetriever::new(
            ctx,
            self.transport.clone(),
            self.secure_channels.clone(),
            self.info.clone(),
            subject.clone(),
            self.scope.clone(),
            self.timing_options,
        );
        debug!(
            "Created RemoteCredentialRetriever for: {}, authority: {}",
            subject, self.info.issuer
        );
        let retriever = Arc::new(retriever);

        registry.insert(subject.clone(), retriever.clone());

        Ok(retriever)
    }
}
