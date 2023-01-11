use crate::authenticated_storage::AuthenticatedStorage;
use crate::{DecryptorWorker, Identity, IdentityVault, SecureChannelRegistry, TrustPolicy};
use ockam_channel::CreateResponderChannelMessage;
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{AsyncTryClone, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<V: IdentityVault, S: AuthenticatedStorage> {
    trust_policy: Arc<dyn TrustPolicy>,
    identity: Identity<V>,
    storage: S,
    registry: SecureChannelRegistry,
}

impl<V: IdentityVault, S: AuthenticatedStorage> IdentityChannelListener<V, S> {
    pub fn new(
        trust_policy: impl TrustPolicy,
        identity: Identity<V>,
        storage: S,
        registry: SecureChannelRegistry,
    ) -> Self {
        IdentityChannelListener {
            trust_policy: Arc::new(trust_policy),
            identity,
            storage,
            registry,
        }
    }
}

#[ockam_core::worker]
impl<V: IdentityVault, S: AuthenticatedStorage> Worker for IdentityChannelListener<V, S> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let trust_policy = Arc::clone(&self.trust_policy);
        let identity = self.identity.async_try_clone().await?;
        DecryptorWorker::create_responder(
            ctx,
            identity,
            self.storage.async_try_clone().await?,
            trust_policy,
            msg,
            self.registry.clone(),
        )
        .await
    }
}
