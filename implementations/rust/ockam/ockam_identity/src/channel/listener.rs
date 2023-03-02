use crate::authenticated_storage::AuthenticatedStorage;
use crate::channel::common::CreateResponderChannelMessage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::{Identity, IdentityVault, SecureChannelListenerTrustOptions};
use ockam_core::compat::boxed::Box;
use ockam_core::sessions::SessionIdLocalInfo;
use ockam_core::{Address, AllowAll, AsyncTryClone, DenyAll, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<V: IdentityVault, S: AuthenticatedStorage> {
    trust_options: SecureChannelListenerTrustOptions,
    identity: Identity<V, S>,
}

impl<V: IdentityVault, S: AuthenticatedStorage> IdentityChannelListener<V, S> {
    pub fn new(trust_options: SecureChannelListenerTrustOptions, identity: Identity<V, S>) -> Self {
        Self {
            trust_options,
            identity,
        }
    }

    pub async fn create(
        ctx: &Context,
        address: Address,
        trust_options: SecureChannelListenerTrustOptions,
        identity: Identity<V, S>,
    ) -> Result<()> {
        if let Some((sessions, session_id)) = &trust_options.session {
            sessions.set_listener_session_id(&address, session_id);
        }

        let listener = Self::new(trust_options, identity);

        ctx.start_worker(
            address, listener, AllowAll, // TODO: @ac allow to customize
            DenyAll,
        )
        .await?;

        Ok(())
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
        let identity = self.identity.async_try_clone().await?;

        // Check if there is a session that connection worker added to LocalInfo
        // If yet - decryptor will be added to that session to be able to receive further messages
        // from the transport connection
        let session_id = SessionIdLocalInfo::find_info(msg.local_message())
            .ok()
            .map(|x| x.session_id().clone());
        let trust_options = self.trust_options.secure_channel_trust_options(session_id);

        DecryptorWorker::create_responder(ctx, identity, trust_options, msg).await
    }
}
