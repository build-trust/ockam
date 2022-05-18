use ockam_core::vault::KeyId;
use ockam_core::{Address, Message};
use serde::{Deserialize, Serialize};

/// Key Exchange completed message
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Message)]
pub struct KeyExchangeCompleted {
    address: Address,
    auth_hash: [u8; 32],
}

impl KeyExchangeCompleted {
    /// Secure Channel address
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Authentication hash
    pub fn auth_hash(&self) -> [u8; 32] {
        self.auth_hash
    }
    /// Constructor
    pub fn new(address: Address, auth_hash: [u8; 32]) -> Self {
        Self { address, auth_hash }
    }
}

pub(crate) struct ChannelKeys {
    pub(crate) key: KeyId,
    pub(crate) nonce: u64,
}

pub(crate) enum Role {
    Initiator,
    Responder,
}

impl Role {
    pub fn role_str(&self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Responder => "responder",
        }
    }
}
