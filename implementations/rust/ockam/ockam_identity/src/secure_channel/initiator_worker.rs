use crate::credential::Credential;
use crate::secure_channel::finalizer::Finalizer;
use crate::secure_channel::initiator_state::State;
use crate::secure_channel::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::secure_channel::packets::{
    EncodedPublicIdentity, FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket,
};
use crate::secure_channel::{Addresses, Role};
use crate::{to_xx_vault, Identity, IdentityError, SecureChannels, TrustContext, TrustPolicy};
use alloc::sync::Arc;
use core::time::Duration;
use ockam_core::{
    Address, AllowAll, Any, DenyAll, LocalOnwardOnly, LocalSourceOnly, Mailbox, Mailboxes,
    NewKeyExchanger, OutgoingAccessControl, Route, Routed,
};
use ockam_core::{Decodable, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::callback::CallbackSender;
use ockam_node::{Context, WorkerBuilder};
use tracing::debug;

pub(crate) struct InitiatorWorker {
    state: Option<State>,
    secure_channels: Arc<SecureChannels>,
    callback_sender: CallbackSender<()>,
}

#[ockam_core::worker]
impl Worker for InitiatorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> ockam_core::Result<()> {
        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        //we initiate the exchange by sending the first packet during the initialization
        match state {
            State::SendPacket1(mut state) => {
                let first_packet = FirstPacket {
                    key_exchange: state.key_exchanger.generate_request(&[]).await?,
                };
                context
                    .send_from_address(
                        state.remote_route.clone(),
                        first_packet,
                        state.addresses.decryptor_remote.clone(),
                    )
                    .await?;
                self.state = Some(State::ReceivePacket2(state.next_state()));
            }
            _ => {
                unreachable!()
            }
        };

        Ok(())
    }

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        if let State::Done(decryptor) = self
            .state
            .as_mut()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?
        {
            //since we cannot replace this worker with the decryptor worker
            //we act as a relay instead
            return decryptor.handle_message(context, message).await;
        }

        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        let new_state = match state {
            State::ReceivePacket2(mut state) => {
                //we only set it once to avoid redirects attack
                state.remote_route = message.return_route();

                let second_packet: SecondPacket =
                    SecondPacket::decode(&message.into_transport_message().payload)?;

                let identity_and_credential = second_packet
                    .key_exchange_with_payload
                    .handle_and_decrypt(&mut state.key_exchanger)
                    .await?;

                let their_identity = identity_and_credential
                    .identity
                    .decode(self.secure_channels.vault())
                    .await?;

                let third_packet = ThirdPacket {
                    key_exchange_with_payload: KeyExchangeWithPayload::create(
                        IdentityAndCredential {
                            identity: EncodedPublicIdentity::from(&state.identity)?,
                            signature: state.signature.clone(),
                            credentials: state.credentials.clone(),
                        },
                        &mut state.key_exchanger,
                    )
                    .await?,
                };

                context
                    .send_from_address(
                        state.remote_route.clone(),
                        third_packet,
                        state.addresses.decryptor_remote.clone(),
                    )
                    .await?;

                let keys = state.key_exchanger.finalize().await?;

                let finalizer = Finalizer {
                    secure_channels: self.secure_channels.clone(),
                    signature: identity_and_credential.signature,
                    identity: state.identity,
                    their_identity,
                    keys,
                    credentials: identity_and_credential.credentials,
                    addresses: state.addresses,
                    remote_route: state.remote_route,
                    trust_context: state.trust_context,
                    trust_policy: state.trust_policy,
                };

                let decryptor = finalizer.finalize(context, Role::Initiator).await?;

                // Notify interested worker about finished init
                self.callback_sender.send(()).await?;

                State::Done(decryptor)
            }

            State::SendPacket1(_) | State::Done(_) => {
                unreachable!()
            }
        };
        self.state = Some(new_state);

        Ok(())
    }
}

impl InitiatorWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        identity: Identity,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credentials: Vec<Credential>,
        trust_context: Option<TrustContext>,
        remote_route: Route,
        timeout: Duration,
    ) -> ockam_core::Result<Address> {
        let (mut callback_waiter, callback_sender) = ockam_node::callback::new_callback();

        let (static_key_id, signature) = secure_channels
            .identities()
            .identities_keys()
            .create_signed_static_key(&identity)
            .await?;

        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(secure_channels.vault()))
            .initiator(Some(static_key_id))
            .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();

        let worker = Self {
            state: Some(State::new(
                remote_route,
                identity,
                addresses.clone(),
                Box::new(key_exchanger),
                trust_policy,
                credentials,
                trust_context,
                signature,
            )),
            secure_channels,
            callback_sender,
        };

        WorkerBuilder::with_mailboxes(
            Self::create_mailboxes(&addresses, decryptor_outgoing_access_control),
            worker,
        )
        .start(context)
        .await?;

        debug!(
            "Starting SecureChannel Initiator at remote: {}",
            &decryptor_remote
        );

        callback_waiter.receive_timeout(timeout).await?;

        Ok(addresses.encryptor)
    }

    pub(crate) fn create_mailboxes(
        addresses: &Addresses,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Mailboxes {
        let remote_mailbox = Mailbox::new(
            addresses.decryptor_remote.clone(),
            // Doesn't matter since we check incoming messages cryptographically,
            // but this may be reduced to allowing only from the transport connection that was used
            // to create this channel initially
            Arc::new(AllowAll),
            // Communicate to the other side of the channel during key exchange
            Arc::new(AllowAll),
        );
        let internal_mailbox = Mailbox::new(
            addresses.decryptor_internal.clone(),
            Arc::new(DenyAll),
            decryptor_outgoing_access_control,
        );
        let api_mailbox = Mailbox::new(
            addresses.decryptor_api.clone(),
            Arc::new(LocalSourceOnly),
            Arc::new(LocalOnwardOnly),
        );

        Mailboxes::new(remote_mailbox, vec![internal_mailbox, api_mailbox])
    }
}
