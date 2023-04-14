use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor::Encryptor;
use crate::channel::v2::initiator_state::State;
use crate::channel::v2::key_exchange_with_payload::KeyExchangeWithPayload;
use crate::channel::v2::packets::{
    EncodedPublicIdentity, FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket,
};
use crate::credential::Credential;
use crate::error::IdentityError;
use crate::{to_symmetric_vault, to_xx_vault, Identity, TrustContext, TrustPolicy};
use ockam_core::{
    AllowAll, KeyExchanger, NewKeyExchanger, OutgoingAccessControl, Route, Routed, Worker,
};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use std::sync::Arc;
use tracing::debug;

pub(super) struct InitiatorWorker {
    state: Option<State>,
}

impl InitiatorWorker {
    pub(super) async fn create(
        context: &Context,
        remote_route: Route,
        identity: Identity,
        addresses: Addresses,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credential: Option<Credential>,
        trust_context: TrustContext,
    ) -> ockam_core::Result<()> {
        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(identity.vault()))
            .initiator()
            .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();

        let worker = Self {
            state: Some(State::new(
                remote_route,
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
            "Starting SecureChannel Initiator v2 at remote: {}",
            &decryptor_remote
        );

        Ok(())
    }
}

#[ockam_core::worker]
impl Worker for InitiatorWorker {
    type Message = SecondPacket;
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> ockam_core::Result<()> {
        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        match state {
            State::SendPacket1(mut state) => {
                let first_packet = FirstPacket {
                    key_exchange: state.key_exchanger.generate_request(&[]).await?,
                };
                context
                    .send(state.remote_route.clone(), first_packet)
                    .await?;
                self.state = Some(State::ReceivePacket2(state.next_state()));
            }
            _ => {
                panic!()
            }
        };

        Ok(())
    }

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        second_packet: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        let new_state = match state {
            State::ReceivePacket2(mut state) => {
                //take the hash before processing the next message, we can validate
                //their signature with it
                let their_hash = state
                    .key_exchanger
                    .current_hash()
                    .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

                let identity_and_credential = second_packet
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

                // since the signature is part of the payload and will influence
                // the hash itself, we cannot use the final hash yet
                let my_hash = state
                    .key_exchanger
                    .current_hash()
                    .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

                let my_signature = state.identity.create_signature(&my_hash, None).await?;

                let third_packet = ThirdPacket {
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
                    .send(state.remote_route.clone(), third_packet)
                    .await?;

                let keys = state.key_exchanger.finalize().await?;
                let encryptor = Encryptor::new(
                    keys.encrypt_key().clone(),
                    0,
                    to_symmetric_vault(state.identity.vault()),
                );

                let decryptor = Decryptor::new(
                    keys.decrypt_key().clone(),
                    to_symmetric_vault(state.identity.vault()),
                );

                //TODO: create encryptor/decryptor workers
                State::Done
            }

            State::SendPacket1(_) => {
                panic!()
            }
            State::Done => {
                panic!()
            }
        };
        self.state = Some(new_state);

        Ok(())
    }
}
