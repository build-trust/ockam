use async_trait::async_trait;
use delegate::delegate;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{VaultForSecureChannels, X25519PublicKey};
use Action::*;
use Event::*;
use Role::*;
use Status::*;

use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::secure_channel::handshake::error::XXError;
use crate::secure_channel::handshake::handshake::Handshake;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, CommonStateMachine, Event, HandshakeKeys, HandshakeResults, IdentityAndCredentials,
    StateMachine, Status,
};
use crate::{Identities, Role, SecureChannelPurposeKey, TrustPolicy};

/// Implementation of a state machine for the key exchange on the responder side
#[async_trait]
impl StateMachine for ResponderStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let state = self.handshake.state.clone();
        match (state.status, event) {
            // Initialize the handshake and wait for message 1
            (Initial, Initialize) => {
                self.initialize_handshake().await?;
                self.handshake.state.status = WaitingForMessage1;
                Ok(NoAction)
            }
            // Process message 1 and send message 2
            (WaitingForMessage1, ReceivedMessage(message)) => {
                self.decode_message1(&message).await?;
                let identity_payload = self
                    .identity_payload
                    .take()
                    .ok_or(XXError::InvalidInternalState)?;
                let message2 = self.encode_message2(&identity_payload).await?;

                self.handshake.state.status = WaitingForMessage3;
                Ok(SendMessage(message2))
            }
            // Process message 3
            (WaitingForMessage3, ReceivedMessage(message)) => {
                let message3_payload = self.decode_message3(&message).await?;
                let their_identity_payload: IdentityAndCredentials =
                    minicbor::decode(&message3_payload)?;
                self.process_identity_payload(
                    their_identity_payload,
                    self.handshake.state.rs()?.clone(),
                )
                .await?;
                self.set_final_state(Responder).await?;
                Ok(NoAction)
            }
            // incorrect state / event
            (s, e) => Err(Error::new(
                Origin::Channel,
                Kind::Invalid,
                format!(
                    "Unexpected combination of responder state and event {:?}/{:?}",
                    s, e
                ),
            )),
        }
    }

    fn get_handshake_results(&self) -> Option<HandshakeResults> {
        self.make_handshake_results(self.get_handshake_keys())
    }
}

pub struct ResponderStateMachine {
    common: CommonStateMachine,
    handshake: Handshake,
    /// this serialized payload contains an identity, its credentials and a signature of its static key
    identity_payload: Option<Vec<u8>>,
}

impl ResponderStateMachine {
    delegate! {
        to self.common {
            async fn process_identity_payload(&mut self, peer: IdentityAndCredentials, peer_public_key: X25519PublicKey) -> Result<()>;
            fn make_handshake_results(&self, handshake_keys: Option<HandshakeKeys>) -> Option<HandshakeResults>;
        }
    }
    delegate! {
        to self.handshake {
            #[call(initialize)]
            async fn initialize_handshake(&mut self) -> Result<()>;
            async fn decode_message1(&mut self, message: &[u8]) -> Result<Vec<u8>>;
            async fn encode_message2(&mut self, payload: &[u8]) -> Result<Vec<u8>>;
            async fn decode_message3(&mut self, message: &[u8]) -> Result<Vec<u8>>;
            async fn set_final_state(&mut self, role: Role) -> Result<()>;
            fn get_handshake_keys(&self) -> Option<HandshakeKeys>;
        }
    }
}

impl ResponderStateMachine {
    pub async fn new(
        vault: Arc<dyn VaultForSecureChannels>,
        identities: Arc<Identities>,
        identifier: Identifier,
        purpose_key: SecureChannelPurposeKey,
        credentials: Vec<CredentialAndPurposeKey>,
        trust_policy: Arc<dyn TrustPolicy>,
        authority: Option<Identifier>,
    ) -> Result<ResponderStateMachine> {
        let common = CommonStateMachine::new(
            identities,
            identifier,
            purpose_key.attestation().clone(),
            credentials,
            trust_policy,
            authority,
        );
        let identity_payload = common.make_identity_payload().await?;

        Ok(ResponderStateMachine {
            common,
            handshake: Handshake::new(vault, purpose_key.key().clone()).await?,
            identity_payload: Some(identity_payload),
        })
    }
}
