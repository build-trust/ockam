use crate::secure_channel::handshake::handshake::HandshakeResults;
use crate::Credential;
use ockam_core::vault::Signature;
use ockam_core::{async_trait, Message, Result};
use serde::{Deserialize, Serialize};

#[async_trait]
pub(super) trait StateMachine: Send + Sync + 'static {
    async fn on_event(&mut self, event: Event) -> Result<Action>;
    fn get_handshake_results(&self) -> Option<HandshakeResults>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Initialize,
    ReceivedMessage(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    NoAction,
    SendMessage(Vec<u8>),
}

/// This internal structure is used as a paylod in the XX protocol
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredentials {
    // exported identity
    pub(super) identity: Vec<u8>,
    // The signature guarantees that the other end has access to the private key of the identity
    // The signature refers to the static key of the noise ('x') and is made with the static
    // key of the identity
    pub(super) signature: Signature,
    pub(super) credentials: Vec<Credential>,
}
