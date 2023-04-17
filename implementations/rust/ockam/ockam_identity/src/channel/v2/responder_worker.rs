use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor::Encryptor;
use crate::channel::encryptor_worker::EncryptorWorker;
use crate::channel::v2::decryptor_worker::DecryptorWorker;
use crate::channel::v2::encryptor_worker;
use crate::channel::v2::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::channel::v2::packets::{
    EncodedPublicIdentity, FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket,
};
use crate::channel::v2::responder_state::{ResponderState, State};
use crate::credential::Credential;
use crate::error::IdentityError;
use crate::{to_symmetric_vault, to_xx_vault, Identity, TrustContext, TrustPolicy};
use ockam_core::{
    AllowAll, Any, DenyAll, LocalOnwardOnly, LocalSourceOnly, Mailbox, Mailboxes, NewKeyExchanger,
    OutgoingAccessControl, Routed,
};
use ockam_core::{Decodable, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::{Context, WorkerBuilder};
use std::sync::Arc;
use tracing::debug;

pub(crate) struct ResponderWorker {
    state: Option<State>,
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

                let my_hash = state
                    .key_exchanger
                    .current_hash()
                    .ok_or(IdentityError::InvalidSecureChannelInternalState)?;

                let my_signature = state.identity.create_signature(&my_hash, None).await?;

                let second_packet = SecondPacket {
                    key_exchange_with_payload: KeyExchangeWithPayload::create(
                        IdentityAndCredential {
                            identity: EncodedPublicIdentity::from(
                                &state.identity.to_public().await?,
                            )?,
                            signature: my_signature,
                            credential: state.credential.take(),
                        },
                        &mut state.key_exchanger,
                    )
                    .await?,
                };

                context
                    .send(state.remote_route.clone(), second_packet)
                    .await?;

                State::DecodeMessage3(state)
            }
            State::DecodeMessage3(mut state) => {
                let third_packet = ThirdPacket::decode(&message.into_transport_message().payload)?;

                let their_hash = state
                    .key_exchanger
                    .current_hash()
                    .ok_or(IdentityError::InvalidSecureChannelInternalState)?;

                let identity_and_credential = third_packet
                    .key_exchange_with_payload
                    .handle_and_decrypt(&mut state.key_exchanger)
                    .await?;

                let their_identity = identity_and_credential
                    .identity
                    .decode(state.identity.vault())
                    .await?;

                their_identity
                    .verify_signature(
                        &identity_and_credential.signature,
                        &their_hash,
                        None,
                        state.identity.vault(),
                    )
                    .await?;

                let keys = state.key_exchanger.finalize().await?;

                // stop the key exchanger worker
                let relay_messages = context.deconstruct(context.address()).await?;

                let decryptor = Decryptor::new(
                    keys.decrypt_key().clone(),
                    to_symmetric_vault(state.identity.vault()),
                );

                let decryptor_worker = DecryptorWorker::new(
                    "responder",
                    state.addresses,
                    decryptor,
                    their_identity.identifier().clone(),
                );

                todo!("mailbox start decryptor worker");
                // context
                //     .wait_for(state.addresses.decryptor_remote.clone())
                //     .await?;
                //
                // for message in relay_messages {
                //     context.forward(message).await?;
                // }
                //
                // let encryptor = Encryptor::new(
                //     keys.encrypt_key().clone(),
                //     0,
                //     to_symmetric_vault(state.identity.vault()),
                // );
                //
                // let encryptor_worker = EncryptorWorker::new(
                //     "responder",
                //     state.addresses,
                //     state.remote_route,
                //     encryptor,
                // );
                //
                // todo!("start encryptor mailbox with right permissions");

                State::Done
            }
            State::Done => {
                panic!()
            }
        };
        self.state = Some(new_state);

        Ok(())
    }
}

impl ResponderWorker {
    pub(crate) async fn create(
        context: &Context,
        identity: Identity,
        addresses: Addresses,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credential: Option<Credential>,
        trust_context: TrustContext,
    ) -> ockam_core::Result<()> {
        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(identity.vault()))
            .responder()
            .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();

        let worker = Self {
            state: Some(State::new(
                identity,
                addresses,
                Box::new(key_exchanger),
                trust_policy,
                credential,
                trust_context,
                decryptor_outgoing_access_control,
            )),
        };

        context
            .start_worker(decryptor_remote.clone(), worker, AllowAll, AllowAll)
            .await?;

        debug!(
            "Starting SecureChannel Responder v2 at remote: {}",
            &decryptor_remote
        );

        Ok(())
    }
}
