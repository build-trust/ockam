use crate::credential::Credential;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::Addresses;
use crate::Identity;
use ockam_core::vault::Signature;
use ockam_core::{route, KeyExchanger, Route};

pub(crate) struct DecodeMessage1 {
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) identity: Identity,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) credentials: Vec<Credential>,
    pub(crate) signature: Signature,
}

impl DecodeMessage1 {
    pub(crate) fn next_state(self) -> DecodeMessage3 {
        DecodeMessage3 {
            key_exchanger: self.key_exchanger,
            identity: self.identity,
            addresses: self.addresses,
            remote_route: self.remote_route,
        }
    }
}

pub(crate) struct DecodeMessage3 {
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,
    pub(crate) identity: Identity,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
}

pub(crate) enum State {
    DecodeMessage1(DecodeMessage1),
    DecodeMessage3(DecodeMessage3),
    Done(DecryptorWorker),
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity: Identity,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        credentials: Vec<Credential>,
        signature: Signature,
    ) -> Self {
        Self::DecodeMessage1(DecodeMessage1 {
            identity,
            signature,
            addresses,
            remote_route: route![],
            key_exchanger,
            credentials,
        })
    }
}
