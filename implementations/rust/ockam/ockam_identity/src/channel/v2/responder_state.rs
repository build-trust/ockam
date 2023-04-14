use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::channel::encryptor;
use crate::channel::encryptor::Encryptor;
use crate::channel::v2::packets::{FirstPacket, IdentityAndCredential, SecondPacket, ThirdPacket};
use crate::credential::Credential;
use crate::error::IdentityError;
use crate::{to_symmetric_vault, Identity, TrustContext, TrustPolicy};
use ockam_core::{
    route, Any, Decodable, KeyExchanger, OutgoingAccessControl, Route, Routed, Worker,
};
use ockam_node::Context;
use std::sync::Arc;

pub(crate) struct ResponderState {
    pub(crate) key_exchanger: Box<dyn KeyExchanger>,

    pub(crate) identity: Identity,
    pub(crate) addresses: Addresses,
    pub(crate) remote_route: Route,
    pub(crate) credential: Option<Credential>,

    // these variables are kept for the next state
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: TrustContext,
    decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

pub(crate) enum State {
    DecodeMessage1(ResponderState),
    DecodeMessage3(ResponderState),
    Done,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity: Identity,
        addresses: Addresses,
        key_exchanger: Box<dyn KeyExchanger>,
        trust_policy: Arc<dyn TrustPolicy>,
        credential: Option<Credential>,
        trust_context: TrustContext,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        Self::DecodeMessage1(ResponderState {
            identity,
            addresses,
            remote_route: route![],
            key_exchanger,
            trust_policy,
            credential,
            trust_context,
            decryptor_outgoing_access_control,
        })
    }
}
