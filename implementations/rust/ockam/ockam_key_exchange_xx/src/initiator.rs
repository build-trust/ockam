use crate::state::State;
use crate::{XXError, XXVault};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};

#[derive(Debug)]
enum InitiatorState {
    EncodeMessage1,
    DecodeMessage2,
    EncodeMessage3,
    Done,
}

/// Represents an XX initiator
#[derive(Debug)]
pub struct Initiator<V: XXVault> {
    state: InitiatorState,
    state_data: State<V>,
}

impl<V: XXVault> Initiator<V> {
    pub(crate) fn new(state_data: State<V>) -> Self {
        Initiator {
            state: InitiatorState::EncodeMessage1,
            state_data,
        }
    }
}

#[async_trait]
impl<V: XXVault> KeyExchanger for Initiator<V> {
    async fn name(&self) -> Result<String> {
        Ok("NOISE_XX".to_string())
    }

    async fn generate_request(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            InitiatorState::EncodeMessage1 => {
                self.state_data.run_prologue().await?;
                let msg = self.state_data.encode_message_1(payload).await?;
                self.state = InitiatorState::DecodeMessage2;
                Ok(msg)
            }
            InitiatorState::EncodeMessage3 => {
                let msg = self.state_data.encode_message_3(payload).await?;
                self.state = InitiatorState::Done;
                Ok(msg)
            }
            InitiatorState::DecodeMessage2 | InitiatorState::Done => {
                Err(XXError::InvalidState.into())
            }
        }
    }

    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            InitiatorState::DecodeMessage2 => {
                let msg = self.state_data.decode_message_2(response).await?;
                self.state = InitiatorState::EncodeMessage3;
                Ok(msg)
            }
            InitiatorState::EncodeMessage1
            | InitiatorState::EncodeMessage3
            | InitiatorState::Done => Err(XXError::InvalidState.into()),
        }
    }

    async fn is_complete(&self) -> Result<bool> {
        Ok(matches!(self.state, InitiatorState::Done))
    }

    async fn finalize(self) -> Result<CompletedKeyExchange> {
        match self.state {
            InitiatorState::Done => self.state_data.finalize_initiator().await,
            _ => Err(XXError::InvalidState.into()),
        }
    }
}
