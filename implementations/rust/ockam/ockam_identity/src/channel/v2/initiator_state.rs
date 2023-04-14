use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor;
use crate::channel::encryptor::Encryptor;
use crate::channel::v2::packets::{
    EncodedPublicIdentity, FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket,
};
use crate::credential::Credential;
use crate::error::IdentityError;
use crate::{to_symmetric_vault, Identity, TrustContext, TrustPolicy};
use ockam_core::{KeyExchanger, OutgoingAccessControl, Route, Routed, Worker};
use ockam_node::Context;
use std::sync::Arc;

pub(super) struct SendPacket1 {
    pub(super) key_exchanger: Box<dyn KeyExchanger>,

    pub(super) identity: Identity,
    pub(super) addresses: Addresses,
    pub(super) remote_route: Route,
    pub(super) credential: Option<Credential>,

    // these variables are kept for the next state
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: TrustContext,
    decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl SendPacket1 {
    pub(super) fn next_state(self) -> ReceivePacket2 {
        ReceivePacket2 {
            key_exchanger: self.key_exchanger,
            identity: self.identity,
            addresses: self.addresses,
            remote_route: self.remote_route,
            credential: self.credential,
            trust_policy: self.trust_policy,
            trust_context: self.trust_context,
            decryptor_outgoing_access_control: self.decryptor_outgoing_access_control,
        }
    }
}

pub(super) struct ReceivePacket2 {
    pub(super) key_exchanger: Box<dyn KeyExchanger>,

    pub(super) identity: Identity,
    pub(super) addresses: Addresses,
    pub(super) remote_route: Route,
    pub(super) credential: Option<Credential>,

    // these variables are kept for the next state
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: TrustContext,
    decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

pub(super) enum State {
    SendPacket1(SendPacket1),
    ReceivePacket2(ReceivePacket2),
    Done,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        remote_route: Route,
        identity: Identity,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        trust_policy: Arc<dyn TrustPolicy>,
        credential: Option<Credential>,
        trust_context: TrustContext,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        Self::SendPacket1(SendPacket1 {
            identity,
            addresses,
            remote_route,
            key_exchanger,
            trust_policy,
            credential,
            trust_context,
            decryptor_outgoing_access_control,
        })
    }
}
