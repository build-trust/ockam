use crate::channel::addresses::Addresses;
use crate::channel::common::CreateResponderChannelMessage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::channel::Role;
use crate::{Identity, SecureChannelListenerTrustOptions};
use ockam_core::compat::boxed::Box;
use ockam_core::sessions::SessionIdLocalInfo;
use ockam_core::{Address, AllowAll, AsyncTryClone, DenyAll, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener {
    trust_options: SecureChannelListenerTrustOptions,
    identity: Identity,
}

impl IdentityChannelListener {
    pub fn new(trust_options: SecureChannelListenerTrustOptions, identity: Identity) -> Self {
        Self {
            trust_options,
            identity,
        }
    }

    pub async fn create(
        ctx: &Context,
        address: Address,
        trust_options: SecureChannelListenerTrustOptions,
        identity: Identity,
    ) -> Result<()> {
        if let Some(ciphertext_session) = &trust_options.consumer_session {
            ciphertext_session.sessions.add_consumer(
                &address,
                &ciphertext_session.session_id,
                ciphertext_session.session_policy,
            );
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
impl Worker for IdentityChannelListener {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let identity = self.identity.async_try_clone().await?;

        // Check if there is a session that connection worker added to LocalInfo
        // If yes - decryptor will be added to that session to be able to receive further messages
        // from the transport connection
        // TODO: Instead look in Sessions struct
        let session_id = SessionIdLocalInfo::find_info(msg.local_message())
            .ok()
            .map(|x| x.session_id().clone());

        let addresses = Addresses::generate(Role::Responder);
        let session_id = self.trust_options.setup_session(&addresses, session_id)?;
        let access_control = self
            .trust_options
            .create_access_control(session_id.clone())?;

        DecryptorWorker::create_responder(
            ctx,
            identity,
            addresses,
            self.trust_options.trust_policy.clone(),
            access_control.decryptor_outgoing_access_control,
            session_id,
            msg,
        )
        .await
    }
}
