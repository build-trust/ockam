//! Credential request/response types

use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetCredentialRequest {
    #[n(1)] overwrite: bool,
    #[n(2)] pub identity_name: Option<String>,
}

impl Encodable for GetCredentialRequest {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for GetCredentialRequest {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl GetCredentialRequest {
    pub fn new(overwrite: bool, identity_name: Option<String>) -> Self {
        Self {
            overwrite,
            identity_name,
        }
    }

    pub fn is_overwrite(&self) -> bool {
        self.overwrite
    }
}

#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PresentCredentialRequest {
    #[b(1)] pub route: String,
    #[n(2)] pub oneway: bool,
}

impl PresentCredentialRequest {
    pub fn new(route: &MultiAddr, oneway: bool) -> Self {
        Self {
            route: route.to_string(),
            oneway,
        }
    }
}
