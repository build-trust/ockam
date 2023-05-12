use crate::secure_channel::handshake::handshake::Handshake;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, CommonStateMachine, Event, HandshakeKeys, HandshakeResults, IdentityAndCredentials,
    StateMachine, Status,
};
use crate::{Credential, Identities, IdentityIdentifier, Role, TrustContext, TrustPolicy, XXVault};
use async_trait::async_trait;
use delegate::delegate;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{KeyId, PublicKey};
use Action::*;
use Event::*;
use Role::*;
use Status::*;

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
                self.decode_message1(message).await?;
                let identity_payload = self.make_identity_payload(&self.handshake.state.s).await?;
                let message2 = self.encode_message2(identity_payload).await?;

                self.handshake.state.status = WaitingForMessage3;
                Ok(SendMessage(message2))
            }
            // Process message 3
            (WaitingForMessage3, ReceivedMessage(message)) => {
                let message3_payload = self.decode_message3(message).await?;
                let their_identity_payload = CommonStateMachine::deserialize(message3_payload)?;
                self.verify_identity(their_identity_payload, &self.handshake.state.rs.clone())
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
}

impl ResponderStateMachine {
    delegate! {
        to self.common {
            async fn make_identity_payload(&self, static_key: &KeyId) -> Result<Vec<u8>>;
            async fn verify_identity(&mut self, peer: IdentityAndCredentials, peer_public_key: &PublicKey) -> Result<()>;
            fn make_handshake_results(&self, handshake_keys: Option<HandshakeKeys>) -> Option<HandshakeResults>;
        }
    }
    delegate! {
        to self.handshake {
            #[call(initialize)]
            async fn initialize_handshake(&mut self) -> Result<()>;
            async fn decode_message1(&mut self, message: Vec<u8>) -> Result<()>;
            async fn encode_message2(&mut self, payload: Vec<u8>) -> Result<Vec<u8>>;
            async fn decode_message3(&mut self, message: Vec<u8>) -> Result<Vec<u8>>;
            async fn set_final_state(&mut self, role: Role) -> Result<()>;
            fn get_handshake_keys(&self) -> Option<HandshakeKeys>;
        }
    }
}

impl ResponderStateMachine {
    pub async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identifier: IdentityIdentifier,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Result<ResponderStateMachine> {
        let common = CommonStateMachine::new(
            vault.clone(),
            identities,
            identifier,
            credentials,
            trust_policy,
            trust_context,
        );
        Ok(ResponderStateMachine {
            common,
            handshake: Handshake::new(vault.clone()).await?,
        })
    }
}
