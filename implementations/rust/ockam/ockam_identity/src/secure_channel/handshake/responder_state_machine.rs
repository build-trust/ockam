use crate::secure_channel::handshake::handshake_state::{Handshake, Status};
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, Event, IdentityAndCredentials, StateMachine,
};
use crate::{Credential, Identities, Identity, TrustContext, TrustPolicy};
use async_trait::async_trait;
use delegate::delegate;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::XXVault;
use Action::*;
use Event::*;
use Status::*;

#[async_trait]
impl StateMachine for ResponderStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let mut state = self.handshake.state.clone();
        match (state.status, event) {
            // Initialize the handshake and wait for message 1
            (Initial, Initialize) => {
                self.initialize_handshake().await?;
                state.status = WaitingForMessage1;
                Ok(NoAction)
            }
            // Process message 1 and send message 2
            (WaitingForMessage1, ReceivedMessage(message)) => {
                self.decode_message1(message).await?;
                let message2 = self.encode_message2().await?;
                state.status = WaitingForMessage3;
                Ok(SendMessage(message2))
            }
            // Process message 3
            (WaitingForMessage3, ReceivedMessage(message)) => {
                let identity_and_credential = self.decode_message3(message).await?;
                let their_identity = self.verify_identity(identity_and_credential).await?;
                self.set_ready_status(their_identity).await?;
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

    fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)> {
        self.get_final_state()
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
            fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)>;
        }
    }

    async fn set_ready_status(&mut self, their_identity: Identity) -> Result<()> {
        let mut state = self.handshake.state.clone();
        // k1, k2 = HKDF(ck, zerolen, 2)
        // k1 is the encryptor key
        // k2 is the decryptor key
        let (k1, k2) = self.handshake.hkdf(&state.ck, &state.k, None).await?;
        state.status = Ready {
            their_identity,
            keys: CompletedKeyExchange::new(state.h, k1, k2),
        };

        Ok(self.handshake.state = state)
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
