use crate::secure_channel::handshake::handshake::Handshake;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, CommonStateMachine, Event, HandshakeKeys, HandshakeResults, IdentityAndCredentials,
    StateMachine, Status,
};
use crate::{Credential, Identities, IdentityIdentifier, Role, TrustContext, TrustPolicy, XXVault};
use delegate::delegate;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_vault::{KeyId, PublicKey};
use Action::*;
use Event::*;
use Role::*;
use Status::*;

/// Implementation of a state machine for the key exchange on the initiator side
#[async_trait]
impl StateMachine for InitiatorStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let state = self.handshake.state.clone();
        match (state.status, event) {
            // Initialize the handshake and send message 1
            (Initial, Initialize) => {
                self.initialize_handshake().await?;
                let message1 = self.encode_message1().await?;

                // Send message 1 and wait for message 2
                self.handshake.state.status = WaitingForMessage2;
                Ok(SendMessage(message1))
            }
            // Process message 2 and send message 3
            (WaitingForMessage2, ReceivedMessage(message)) => {
                let message2_payload = self.decode_message2(message).await?;
                let their_identity_payload = CommonStateMachine::deserialize(message2_payload)?;
                self.verify_identity(their_identity_payload, &self.handshake.state.rs.clone())
                    .await?;
                let identity_payload = self.make_identity_payload(&self.handshake.state.s).await?;
                let message3 = self.encode_message3(identity_payload).await?;
                self.set_final_state(Initiator).await?;
                Ok(SendMessage(message3))
            }
            // incorrect state / event
            (s, e) => Err(Error::new(
                Origin::Channel,
                Kind::Invalid,
                format!(
                    "Unexpected combination of initiator state and event {:?}/{:?}",
                    s, e
                ),
            )),
        }
    }

    fn get_handshake_results(&self) -> Option<HandshakeResults> {
        self.make_handshake_results(self.get_handshake_keys())
    }
}

/// Implementation of the state machine actions, delegated to the Handshake module
pub(super) struct InitiatorStateMachine {
    pub(super) common: CommonStateMachine,
    pub(super) handshake: Handshake,
}

impl InitiatorStateMachine {
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
            async fn encode_message1(&mut self) -> Result<Vec<u8>>;
            async fn decode_message2(&mut self, message: Vec<u8>) -> Result<Vec<u8>>;
            async fn encode_message3(&mut self, payload: Vec<u8>) -> Result<Vec<u8>>;
            async fn set_final_state(&mut self, role: Role) -> Result<()>;
            fn get_handshake_keys(&self) -> Option<HandshakeKeys>;
        }
    }
}

impl InitiatorStateMachine {
    pub async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identifier: IdentityIdentifier,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Result<InitiatorStateMachine> {
        let common = CommonStateMachine::new(
            vault.clone(),
            identities,
            identifier,
            credentials,
            trust_policy,
            trust_context,
        );
        Ok(InitiatorStateMachine {
            common,
            handshake: Handshake::new(vault.clone()).await?,
        })
    }
}
