use crate::credential::Credential;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::Addresses;
use crate::{IdentityIdentifier, TrustContext, TrustPolicy};
use ockam_core::vault::Signature;
use ockam_core::{KeyExchanger, Route};
use std::sync::Arc;

pub(super) struct SendPacket1 {
    pub(super) key_exchanger: Box<dyn KeyExchanger>,
    pub(super) identifier: IdentityIdentifier,
    pub(super) addresses: Addresses,
    pub(super) remote_route: Route,
    pub(super) credentials: Vec<Credential>,

    // these variables are kept for the next state
    signature: Signature,
    trust_context: Option<TrustContext>,
    trust_policy: Arc<dyn TrustPolicy>,
}

impl SendPacket1 {
    pub(super) fn next_state(self) -> ReceivePacket2 {
        ReceivePacket2 {
            key_exchanger: self.key_exchanger,
            identifier: self.identifier,
            addresses: self.addresses,
            remote_route: self.remote_route,
            credentials: self.credentials,
            signature: self.signature,
            trust_context: self.trust_context,
            trust_policy: self.trust_policy,
        }
    }
}

pub(super) struct ReceivePacket2 {
    pub(super) key_exchanger: Box<dyn KeyExchanger>,
    pub(super) identifier: IdentityIdentifier,
    pub(super) addresses: Addresses,
    pub(super) remote_route: Route,
    pub(super) credentials: Vec<Credential>,
    pub(super) signature: Signature,
    pub(crate) trust_context: Option<TrustContext>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

pub(super) enum State {
    SendPacket1(SendPacket1),
    ReceivePacket2(ReceivePacket2),
    Done(DecryptorWorker),
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        remote_route: Route,
        identifier: IdentityIdentifier,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        trust_policy: Arc<dyn TrustPolicy>,
        credentials: Vec<Credential>,
        trust_context: Option<TrustContext>,
        signature: Signature,
    ) -> Self {
        Self::SendPacket1(SendPacket1 {
            identifier,
            signature,
            addresses,
            remote_route,
            key_exchanger,
            trust_policy,
            credentials,
            trust_context,
        })
    }
}
