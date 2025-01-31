use super::RoleInShare;
use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use serde::Serialize;

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Message)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptInvitation {
    #[n(1)] pub id: String,
}

impl Encodable for AcceptInvitation {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for AcceptInvitation {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Message)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptedInvitation {
    #[n(1)] pub id: String,
    #[n(2)] pub scope: RoleInShare,
    #[n(3)] pub target_id: String,
}

impl Encodable for AcceptedInvitation {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for AcceptedInvitation {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
