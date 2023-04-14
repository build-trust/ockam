use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor::Encryptor;
use crate::channel::Role;
use crate::credential::Credential;
use crate::{Identity, IdentityIdentifier, PublicIdentity, TrustContext, TrustPolicy};
use alloc::vec::Vec;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{route, Message};
use ockam_core::{Address, KeyExchanger, Route};
use serde::{Deserialize, Serialize};

pub(crate) struct IdentityExchangeState {
    pub(crate) role: Role,
    pub(crate) identity: Identity,
    pub(crate) encryptor: Option<Encryptor>,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) decryptor: Decryptor,
    pub(crate) auth_hash: [u8; 32],
    pub(crate) identity_sent: bool,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,

    pub(crate) credential: Option<Credential>,
    trust_context: TrustContext,
}

//temporary state for credential exchange
//to be removed
pub(crate) struct CredentialExchangeState {
    //for debug purposes only
    pub(crate) role: Role,
    pub(crate) addresses: Addresses,
    pub(crate) decryptor: Decryptor,
    pub(crate) their_identity_id: IdentityIdentifier,
    pub(crate) identity: Identity,
    pub(crate) trust_context: TrustContext,
    pub(crate) remote_route: Route,
    pub(crate) encryptor: Option<Encryptor>,
}

pub(crate) struct InitializedState {
    //for debug purposes only
    pub(crate) role: &'static str,
    pub(crate) addresses: Addresses,
    pub(crate) decryptor: Decryptor,
    pub(crate) their_identity_id: IdentityIdentifier,
}

pub(crate) struct InitializerStatus {
    pub(crate) identity: Identity,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) role: Role,
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) initialization_run: bool,

    // these variables are kept for the next state
    trust_policy: Arc<dyn TrustPolicy>,
    credential: Option<Credential>,
    trust_context: TrustContext,
}

impl InitializerStatus {
    pub(crate) fn into_identity_exchange(
        self,
        encryptor: Encryptor,
        decryptor: Decryptor,
        auth_hash: [u8; 32],
    ) -> IdentityExchangeState {
        IdentityExchangeState {
            role: self.role,
            identity: self.identity,
            remote_route: self.remote_route,
            addresses: self.addresses,
            trust_policy: self.trust_policy,
            encryptor: Some(encryptor),
            decryptor,
            auth_hash,
            identity_sent: false,
            credential: self.credential,
            trust_context: self.trust_context,
        }
    }
}

impl IdentityExchangeState {
    pub(crate) fn into_credential_exchange(
        self,
        their_identity_id: IdentityIdentifier,
    ) -> CredentialExchangeState {
        CredentialExchangeState {
            role: self.role,
            addresses: self.addresses,
            decryptor: self.decryptor,
            identity: self.identity,
            trust_context: self.trust_context,
            remote_route: self.remote_route,
            encryptor: self.encryptor,
            their_identity_id,
        }
    }
}

impl CredentialExchangeState {
    pub(crate) fn into_initialized(self) -> InitializedState {
        InitializedState {
            role: self.role.str(),
            addresses: self.addresses,
            decryptor: self.decryptor,
            their_identity_id: self.their_identity_id,
        }
    }
}

// false positive: https://github.com/rust-lang/rust-clippy/issues/9798
#[allow(clippy::large_enum_variant)]
pub(crate) enum State {
    KeyExchange(InitializerStatus),
    IdentityExchange(IdentityExchangeState),
    CredentialExchange(CredentialExchangeState),
    Initialized(InitializedState),
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        role: Role,
        identity: Identity,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        remote_route: Route,
        trust_policy: Arc<dyn TrustPolicy>,
        credential: Option<Credential>,
        trust_context: TrustContext,
    ) -> Self {
        Self::KeyExchange(InitializerStatus {
            role,
            identity,
            addresses,
            remote_route,
            key_exchanger,
            trust_policy,
            initialization_run: true,
            credential,
            trust_context,
        })
    }
}
