use crate::state::State;
use crate::{XXError, XXVault};
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

impl<V: XXVault> KeyExchanger for Responder<V> {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
        match self.state {
            ResponderState::DecodeMessage1 => {
                self.state_data.run_prologue()?;
                let msg = self.state_data.decode_message_1(data)?;
                self.state = ResponderState::EncodeMessage2;
                Ok(msg)
            }
            ResponderState::EncodeMessage2 => {
                let msg = self.state_data.encode_message_2(data)?;
                self.state = ResponderState::DecodeMessage3;
                Ok(msg)
            }
            ResponderState::DecodeMessage3 => {
                let msg = self.state_data.decode_message_3(data)?;
                self.state = ResponderState::Done;
                Ok(msg)
            }
            ResponderState::Done => Err(XXError::InvalidState.into()),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        match self.state {
            ResponderState::Done => self.state_data.finalize_responder(),
            _ => Err(XXError::InvalidState.into()),
        }
    }
}
