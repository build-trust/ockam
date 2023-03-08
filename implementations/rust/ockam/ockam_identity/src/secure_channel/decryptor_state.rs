use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::{Addresses, Role};
use crate::{Identity, IdentityIdentifier, SecureChannels, TrustPolicy};
use alloc::vec::Vec;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, KeyExchanger, Route};

pub(crate) struct KeyExchangeState {
    pub(crate) role: Role,
    pub(crate) identity: Identity,
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) initial_responder_payload: Option<Vec<u8>>,
    pub(crate) initialization_run: bool,

    remote_backwards_compatibility_address: Option<Address>,
    trust_policy: Arc<dyn TrustPolicy>,
}

pub(crate) struct IdentityExchangeState {
    pub(crate) role: Role,
    pub(crate) identity: Identity,
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) encryptor: Option<Encryptor>,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) decryptor: Decryptor,
    pub(crate) auth_hash: [u8; 32],
    pub(crate) identity_sent: bool,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
    pub(crate) remote_backwards_compatibility_address: Option<Address>,
}

pub(crate) struct InitializedState {
    //for debug purposes only
    pub(crate) role: &'static str,
    pub(crate) addresses: Addresses,
    pub(crate) decryptor: Decryptor,
    pub(crate) their_identity_id: IdentityIdentifier,
}

impl KeyExchangeState {
    pub(crate) fn into_identity_exchange(
        self,
        encryptor: Encryptor,
        decryptor: Decryptor,
        auth_hash: [u8; 32],
    ) -> IdentityExchangeState {
        IdentityExchangeState {
            role: self.role,
            identity: self.identity,
            secure_channels: self.secure_channels,
            remote_route: self.remote_route,
            addresses: self.addresses,
            trust_policy: self.trust_policy,
            remote_backwards_compatibility_address: self.remote_backwards_compatibility_address,
            encryptor: Some(encryptor),
            decryptor,
            auth_hash,
            identity_sent: false,
        }
    }
}

impl IdentityExchangeState {
    pub(crate) fn into_initialized(
        self,
        their_identity_id: IdentityIdentifier,
    ) -> InitializedState {
        InitializedState {
            role: self.role.str(),
            addresses: self.addresses,
            decryptor: self.decryptor,
            their_identity_id,
        }
    }
}

// false positive: https://github.com/rust-lang/rust-clippy/issues/9798
#[allow(clippy::large_enum_variant)]
pub(crate) enum State {
    KeyExchange(KeyExchangeState),
    IdentityExchange(IdentityExchangeState),
    Initialized(InitializedState),
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        role: Role,
        identity: Identity,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        remote_route: Route,
        trust_policy: Arc<dyn TrustPolicy>,
        remote_backwards_compatibility_address: Option<Address>,
        initial_responder_payload: Option<Vec<u8>>,
    ) -> Self {
        Self::KeyExchange(KeyExchangeState {
            role,
            identity,
            secure_channels,
            addresses,
            remote_route,
            key_exchanger,
            trust_policy,
            remote_backwards_compatibility_address,
            initial_responder_payload,
            initialization_run: true,
        })
    }
}
