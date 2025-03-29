use super::{InvitationWithAccess, ReceivedInvitation, SentInvitation};
use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Message)]
#[cbor(map)]
#[rustfmt::skip]
pub struct ListInvitations {
    #[n(1)] pub kind: InvitationListKind,
}

impl Encodable for ListInvitations {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ListInvitations {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Clone, Debug, PartialEq, Decode, Encode, CborLen, Deserialize, Serialize)]
#[cbor(index_only)]
#[rustfmt::skip]
pub enum InvitationListKind {
    #[n(0)] All,
    #[n(1)] Sent,
    #[n(2)] Received,
    #[n(3)] Accepted,
}

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Message)]
#[cbor(map)]
#[rustfmt::skip]
pub struct InvitationList {
    #[n(1)] pub sent: Option<Vec<SentInvitation>>,
    #[n(2)] pub received: Option<Vec<ReceivedInvitation>>,
    #[n(3)] pub accepted: Option<Vec<InvitationWithAccess>>,
}

impl Encodable for InvitationList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for InvitationList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
