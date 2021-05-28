use crate::hidden_message::HiddenMessage;
use crate::message::Message;

/// A message classification by the prover
#[derive(Copy, Clone, Debug)]
pub enum ProofMessage {
    /// Message will be revealed to a verifier
    Revealed(Message),
    /// Message will be hidden from a verifier
    Hidden(HiddenMessage),
}

impl ProofMessage {
    /// Extract the internal message
    pub fn get_message(&self) -> Message {
        match *self {
            ProofMessage::Revealed(r) => r,
            ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(p)) => p,
            ProofMessage::Hidden(HiddenMessage::ExternalBlinding(p, _)) => p,
        }
    }
}
