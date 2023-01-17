use crate::authenticated_storage::AuthenticatedStorage;
use crate::channel::common::CreateResponderChannelMessage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::{Identity, IdentityVault, TrustPolicy};
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{AsyncTryClone, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<V: IdentityVault, S: AuthenticatedStorage> {
    trust_policy: Arc<dyn TrustPolicy>,
    identity: Identity<V, S>,
}

impl<V: IdentityVault, S: AuthenticatedStorage> IdentityChannelListener<V, S> {
    pub fn new(trust_policy: impl TrustPolicy, identity: Identity<V, S>) -> Self {
        IdentityChannelListener {
            trust_policy: Arc::new(trust_policy),
            identity,
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
        DecryptorWorker::create_responder(ctx, identity, trust_policy, msg).await
    }
}
