use crate::state::State;
use crate::{XXError, XXVault};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::error::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};

#[derive(Debug)]
enum ResponderState {
    DecodeMessage1,
    EncodeMessage2,
    DecodeMessage3,
    Done,
}

/// Represents an XX responder
#[derive(Debug)]
pub struct Responder<V: XXVault> {
    state: ResponderState,
    state_data: State<V>,
}

impl<V: XXVault> Responder<V> {
    pub(crate) fn new(state_data: State<V>) -> Self {
        Responder {
            state: ResponderState::DecodeMessage1,
            state_data,
        }
    }
}

#[async_trait]
impl<V: XXVault> KeyExchanger for Responder<V> {
    async fn name(&self) -> Result<String> {
        Ok("NOISE_XX".to_string())
    }

    async fn generate_request(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            ResponderState::EncodeMessage2 => {
                let msg = self.state_data.encode_message_2(payload).await?;
                self.state = ResponderState::DecodeMessage3;
                Ok(msg)
            }
            ResponderState::DecodeMessage1
            | ResponderState::DecodeMessage3
            | ResponderState::Done => Err(XXError::InvalidState.into()),
        }
    }

    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            ResponderState::DecodeMessage1 => {
                self.state_data.run_prologue().await?;
                let msg = self.state_data.decode_message_1(response).await?;
                self.state = ResponderState::EncodeMessage2;
                Ok(msg)
            }
            ResponderState::DecodeMessage3 => {
                let msg = self.state_data.decode_message_3(response).await?;
                self.state = ResponderState::Done;
                Ok(msg)
            }
            ResponderState::EncodeMessage2 | ResponderState::Done => {
                Err(XXError::InvalidState.into())
            }
        }
    }

    async fn is_complete(&self) -> Result<bool> {
        Ok(matches!(self.state, ResponderState::Done))
    }

    async fn finalize(self) -> Result<CompletedKeyExchange> {
        match self.state {
            ResponderState::Done => self.state_data.finalize_responder().await,
            _ => Err(XXError::InvalidState.into()),
        }
    }
}
