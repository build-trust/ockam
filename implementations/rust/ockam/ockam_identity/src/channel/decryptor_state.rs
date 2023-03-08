use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor::Encryptor;
use crate::IdentityIdentifier;
use ockam_core::compat::boxed::Box;
use ockam_core::KeyExchanger;

pub(crate) struct KeyExchange {
    pub key_exchanger: Box<dyn KeyExchanger>,
}

pub(crate) struct ExchangeIdentity {
    pub encryptor: Encryptor,
    pub decryptor: Decryptor,
    pub auth_hash: [u8; 32],
    pub identity_sent: bool,
    pub received_identity_id: Option<IdentityIdentifier>,
}

pub(crate) struct Initialized {
    pub decryptor: Decryptor,
    pub their_identity_id: IdentityIdentifier,
}

pub(crate) enum State {
    KeyExchange,
    ExchangeIdentity,
    Initialized,
}
