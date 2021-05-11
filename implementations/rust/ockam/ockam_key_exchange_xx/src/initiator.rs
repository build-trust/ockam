use crate::state::State;
use crate::{XXError, XXVault};
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

impl<V: XXVault> KeyExchanger for Initiator<V> {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
        match self.state {
            InitiatorState::EncodeMessage1 => {
                self.state_data.run_prologue()?;
                let msg = self.state_data.encode_message_1(data)?;
                self.state = InitiatorState::DecodeMessage2;
                Ok(msg)
            }
            InitiatorState::DecodeMessage2 => {
                let msg = self.state_data.decode_message_2(data)?;
                self.state = InitiatorState::EncodeMessage3;
                Ok(msg)
            }
            InitiatorState::EncodeMessage3 => {
                let msg = self.state_data.encode_message_3(data)?;
                self.state = InitiatorState::Done;
                Ok(msg)
            }
            InitiatorState::Done => Err(XXError::InvalidState.into()),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, InitiatorState::Done)
    }

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        match self.state {
            InitiatorState::Done => self.state_data.finalize_initiator(),
            _ => Err(XXError::InvalidState.into()),
        }
    }
}
