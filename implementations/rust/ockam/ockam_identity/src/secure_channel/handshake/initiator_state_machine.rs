use crate::secure_channel::handshake::handshake_state::{Handshake, Status};
use crate::secure_channel::handshake::handshake_state_machine::Action::SendMessage;
use crate::secure_channel::handshake::handshake_state_machine::Event::ReceivedMessage;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, Event, IdentityAndCredentials, StateMachine,
};
use crate::{Credential, Identities, Identity, TrustContext, TrustPolicy};
use delegate::delegate;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::XXVault;
use Event::*;
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

                // Wait for message 2 and send message 1
                state.status = WaitingForMessage2;
                Ok(SendMessage(message1))
            }
            // Process message 2 and send message 3
            (WaitingForMessage2, ReceivedMessage(message)) => {
                let identity_and_credential = self.decode_message2(message).await?;
                let their_identity = self.verify_identity(identity_and_credential).await?;

                let message3 = self.encode_message3().await?;
                self.set_ready_status(their_identity).await?;
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

    fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)> {
        self.get_final_state()
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
            pub fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)>;
        }
    }

    async fn set_ready_status(&mut self, their_identity: Identity) -> Result<()> {
        let mut state = self.handshake.state.clone();
        // k1, k2 = HKDF(ck, zerolen, 2)
        // k2 is the encryptor key
        // k1 is the decryptor key
        let (k1, k2) = self.handshake.hkdf(&state.ck, &state.k, None).await?;
        state.status = Ready {
            their_identity,
            keys: CompletedKeyExchange::new(state.h, k2, k1),
        };

        Ok(self.handshake.state = state)
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
