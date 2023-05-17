use crate::secure_channel::handshake::handshake::{Handshake, HandshakeResults, Status};
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, Event, IdentityAndCredentials, StateMachine,
};
use crate::{Credential, Identities, Identity, Role, TrustContext, TrustPolicy, XXVault};
use delegate::delegate;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use Action::*;
use Event::*;
use Role::*;
use Status::*;

#[async_trait]
impl StateMachine for InitiatorStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let mut state = self.handshake.state.clone();
        match (state.status, event) {
            // Initialize the handshake and send message 1
            (Initial, Initialize) => {
                self.initialize_handshake().await?;
                let message1 = self.encode_message1().await?;

                // Send message 1 and wait for message 2
                state.status = WaitingForMessage2;
                Ok(SendMessage(message1))
            }
            // Process message 2 and send message 3
            (WaitingForMessage2, ReceivedMessage(message)) => {
                let identity_and_credential = self.decode_message2(message).await?;
                let their_identity = self.verify_identity(identity_and_credential).await?;

                let message3 = self.encode_message3().await?;
                self.set_final_state(their_identity, Initiator).await?;
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
        self.get_handshake_results()
    }
}

pub(super) struct InitiatorStateMachine {
    pub(super) handshake: Handshake,
}

impl InitiatorStateMachine {
    delegate! {
        to self.handshake {
            #[call(initialize)]
            async fn initialize_handshake(&mut self) -> Result<()>;
            async fn encode_message1(&mut self) -> Result<Vec<u8>>;
            async fn decode_message2(&mut self, message: Vec<u8>) -> Result<IdentityAndCredentials>;
            async fn encode_message3(&mut self) -> Result<Vec<u8>>;
            async fn verify_identity(&self, identity_and_credential: IdentityAndCredentials) -> Result<Identity>;
            async fn set_final_state(&mut self, their_identity: Identity, role: Role) -> Result<()>;
            fn get_handshake_results(&self) -> Option<HandshakeResults>;
        }
    }
}

impl InitiatorStateMachine {
    pub async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Result<InitiatorStateMachine> {
        Ok(InitiatorStateMachine {
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
