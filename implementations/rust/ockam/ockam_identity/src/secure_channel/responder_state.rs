use crate::credential::Credential;
use crate::secure_channel::completer::ExchangeCompleter;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::{Addresses, Role};
use crate::{Identity, IdentityIdentifier, TrustContext, TrustPolicy};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use ockam_core::vault::Signature;
use ockam_core::{route, CompletedKeyExchange, KeyExchanger, Route};

pub(crate) struct DecodeMessage1 {
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) identity_identifier: IdentityIdentifier,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) credentials: Vec<Credential>,
    pub(crate) signature: Signature,

    // these variables are kept for the next state
    trust_context: Option<TrustContext>,
    trust_policy: Arc<dyn TrustPolicy>,
}

impl DecodeMessage1 {
    pub(crate) fn next_state(self) -> DecodeMessage3 {
        DecodeMessage3 {
            key_exchanger: self.key_exchanger,
            identity_identifier: self.identity_identifier,
            addresses: self.addresses,
            remote_route: self.remote_route,
            trust_context: self.trust_context,
            trust_policy: self.trust_policy,
        }
    }
}

pub(crate) struct DecodeMessage3 {
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) identity_identifier: IdentityIdentifier,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl DecodeMessage3 {
    pub(crate) fn into_completer(
        self,
        keys: CompletedKeyExchange,
        their_identity: Identity,
        their_signature: Signature,
        their_credentials: Vec<Credential>,
    ) -> ExchangeCompleter {
        ExchangeCompleter {
            role: Role::Responder,
            identity_identifier: self.identity_identifier,
            keys,
            their_signature,
            their_identity,
            their_credentials,
            addresses: self.addresses,
            remote_route: self.remote_route,
            trust_context: self.trust_context,
            trust_policy: self.trust_policy,
        }
    }
}

pub(crate) enum State {
    DecodeMessage1(DecodeMessage1),
    DecodeMessage3(DecodeMessage3),
    Done(DecryptorWorker),
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity_identifier: IdentityIdentifier,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        credentials: Vec<Credential>,
        signature: Signature,
        trust_context: Option<TrustContext>,
        trust_policy: Arc<dyn TrustPolicy>,
    ) -> Self {
        Self::DecodeMessage1(DecodeMessage1 {
            identity_identifier,
            signature,
            addresses,
            remote_route: route![],
            key_exchanger,
            credentials,
            trust_context,
            trust_policy,
        })
    }
}
