use ockam_core::{KeyExchanger, NewKeyExchanger};

#[derive(Clone)]
pub(crate) enum Role {
    Initiator,
    Responder,
}

impl Role {
    pub fn is_initiator(&self) -> bool {
        match self {
            Role::Initiator => true,
            Role::Responder => false,
        }
    }

    pub fn str(&self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Responder => "responder",
        }
    }
}

/// KeyExchanger with extra constraints
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + Sync + 'static {}

/// NewKeyExchanger with extra constraints
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + Sync + 'static {}
