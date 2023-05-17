use crate::Identity;
use ockam_core::vault::KeyId;
use ockam_core::{async_trait, Result};

/// Interface for a state machine in a key exchange protocol
#[async_trait]
pub(super) trait StateMachine: Send + Sync + 'static {
    async fn on_event(&mut self, event: Event) -> Result<Action>;
    fn get_handshake_results(&self) -> Option<HandshakeResults>;
}

/// Events received by the state machine, either initializing the state machine
/// or receiving a message from the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Event {
    Initialize,
    ReceivedMessage(Vec<u8>),
}

/// Outcome of processing an event: either no action or a message to send to the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Action {
    NoAction,
    SendMessage(Vec<u8>),
}

/// List of possible states for the initiator or responder sides of the exchange
#[derive(Debug, Clone)]
pub(super) enum Status {
    Initial,
    WaitingForMessage1,
    WaitingForMessage2,
    WaitingForMessage3,
    Ready(HandshakeResults),
}

/// The end result of a handshake is a pair of encryption/decryption keys +
/// the identity of the other party
#[derive(Debug, Clone)]
pub(super) struct HandshakeResults {
    pub(super) encryption_key: KeyId,
    pub(super) decryption_key: KeyId,
    pub(super) their_identity: Identity,
}
