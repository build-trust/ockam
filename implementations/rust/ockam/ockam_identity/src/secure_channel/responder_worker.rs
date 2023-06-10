use crate::credential::Credential;
use crate::secure_channel::initiator_worker::InitiatorWorker;
use crate::secure_channel::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::secure_channel::packets::{
    EncodedPublicIdentity, FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket,
};
use crate::secure_channel::responder_state::State;
use crate::secure_channel::Addresses;
use crate::{
    to_xx_vault, IdentityError, IdentityIdentifier, SecureChannels, TrustContext, TrustPolicy,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use ockam_core::{Address, Any, NewKeyExchanger, OutgoingAccessControl, Routed};
use ockam_core::{Decodable, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::{Context, WorkerBuilder};
use tracing::debug;

pub(crate) struct ResponderWorker {
    state: Option<State>,

    //only reference to the implementation, not part of the state
    secure_channels: Arc<SecureChannels>,
}

#[ockam_core::worker]
impl Worker for ResponderWorker {
    type Message = Any;
    type Context = Context;

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
            State::DecodeMessage1(mut state) => {
                //we only set it once to avoid redirects attack
                state.remote_route = message.return_route();

                let first_packet = FirstPacket::decode(&message.into_transport_message().payload)?;

                //ignoring output since no payload is expected in the first packet
                let _ = state
                    .key_exchanger
                    .handle_response(&first_packet.key_exchange)
                    .await?;

                let identity = self
                    .secure_channels
                    .identities
                    .identities_repository
                    .get_identity(&state.identity_identifier)
                    .await?;

                let second_packet = SecondPacket {
                    key_exchange_with_payload: KeyExchangeWithPayload::create(
                        IdentityAndCredential {
                            identity: EncodedPublicIdentity::from(&identity)?,
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
                        second_packet,
                        state.addresses.decryptor_remote.clone(),
                    )
                    .await?;

                State::DecodeMessage3(state.next_state())
            }
            State::DecodeMessage3(mut state) => {
                let third_packet = ThirdPacket::decode(&message.into_transport_message().payload)?;

                let identity_and_credential = third_packet
                    .key_exchange_with_payload
                    .handle_and_decrypt(&mut state.key_exchanger)
                    .await?;

                //the identity has not been verified yet
                let their_identity = identity_and_credential
                    .identity
                    .decode(
                        self.secure_channels.identities.repository(),
                        self.secure_channels.vault(),
                    )
                    .await?;

                let keys = state.key_exchanger.finalize().await?;

                let decryptor = state
                    .into_completer(
                        keys,
                        their_identity,
                        identity_and_credential.signature,
                        identity_and_credential.credentials,
                    )
                    .complete(context, self.secure_channels.clone())
                    .await?;

                State::Done(decryptor)
            }
            State::Done(_) => {
                unreachable!()
            }
        };
        self.state = Some(new_state);

        Ok(())
    }
}

impl ResponderWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        identity_identifier: IdentityIdentifier,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credentials: Vec<Credential>,
        trust_context: Option<TrustContext>,
    ) -> ockam_core::Result<Address> {
        let identity = secure_channels
            .identities
            .identities_repository
            .get_identity(&identity_identifier)
            .await?;

        let (static_key_id, signature) = secure_channels
            .identities()
            .identities_keys()
            .create_signed_static_key(&identity)
            .await?;

        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(secure_channels.vault()))
            .responder(Some(static_key_id))
            .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();

        let worker = Self {
            state: Some(State::new(
                identity_identifier,
                addresses.clone(),
                Box::new(key_exchanger),
                credentials,
                signature,
                trust_context,
                trust_policy,
            )),
            secure_channels,
        };

        WorkerBuilder::new(worker)
            .with_mailboxes(InitiatorWorker::create_mailboxes(
                &addresses,
                decryptor_outgoing_access_control,
            ))
            .start(context)
            .await?;

        debug!(
            "Starting SecureChannel Responder at remote: {}",
            &decryptor_remote
        );

        //wait until the worker is ready to receive messages
        context.wait_for(addresses.decryptor_remote.clone()).await?;
        Ok(addresses.decryptor_remote)
    }
}
