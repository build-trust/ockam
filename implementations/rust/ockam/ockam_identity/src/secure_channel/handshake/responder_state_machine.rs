use crate::secure_channel::handshake::handshake::{Handshake, IdentityAndCredentials};
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, Event, HandshakeResults, StateMachine, Status,
};
use crate::{Credential, Identities, Identity, Role, TrustContext, TrustPolicy, XXVault};
use async_trait::async_trait;
use delegate::delegate;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
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
                let message2 = self.encode_message2().await?;
                self.handshake.state.status = WaitingForMessage3;
                Ok(SendMessage(message2))
            }
            // Process message 3
            (WaitingForMessage3, ReceivedMessage(message)) => {
                let identity_and_credential = self.decode_message3(message).await?;
                let their_identity = self.verify_identity(identity_and_credential).await?;
                self.set_final_state(their_identity, Responder).await?;
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
    /// Implementation of the state machine actions, delegated to the Handshake module
    fn get_handshake_results(&self) -> Option<HandshakeResults> {
        self.get_handshake_results()
    }
}

pub struct ResponderStateMachine {
    handshake: Handshake,
}

impl ResponderStateMachine {
    delegate! {
        to self.handshake {
            #[call(initialize)]
            async fn initialize_handshake(&mut self) -> Result<()>;
            async fn decode_message1(&mut self, message: Vec<u8>) -> Result<()>;
            async fn encode_message2(&mut self) -> Result<Vec<u8>>;
            async fn decode_message3(&mut self, message: Vec<u8>) -> Result<IdentityAndCredentials>;
            async fn verify_identity(&self, identity_and_credential: IdentityAndCredentials) -> Result<Identity>;
            async fn set_final_state(&mut self, their_identity: Identity, role: Role) -> Result<()>;
            fn get_handshake_results(&self) -> Option<HandshakeResults>;
        }
    }
}

impl ResponderStateMachine {
    pub async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Result<ResponderStateMachine> {
        Ok(ResponderStateMachine {
            handshake: Handshake::new(
                vault,
                identities,
                identity,
                credentials,
                trust_policy,
                trust_context,
            )
            .await?,
        })
    }
}
