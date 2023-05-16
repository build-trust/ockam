use crate::secure_channel::handshake::handshake_state::HandshakeResults;
use crate::{Credential, Identity};
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

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredentials {
    pub(super) identity: EncodedPublicIdentity,
    // The signature guarantees that the other end has access to the private key of the identity
    // The signature refers to the static key of the noise ('x') and is made with the static
    // key of the identity
    pub(super) signature: Signature,
    pub(super) credentials: Vec<Credential>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct EncodedPublicIdentity {
    pub(super) encoded: Vec<u8>,
}

impl EncodedPublicIdentity {
    pub(super) fn from(public_identity: &Identity) -> Result<Self> {
        Ok(Self {
            encoded: public_identity.export()?,
        })
    }
}
