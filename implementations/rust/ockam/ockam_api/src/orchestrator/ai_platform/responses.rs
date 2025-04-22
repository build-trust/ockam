use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Zone {
    #[n(1)] pub zone: String,
    #[n(2)] pub cluster: String,
}

impl Encodable for Zone {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for Zone {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct ZoneList(#[n(0)] pub(crate) Vec<Zone>);

impl Encodable for ZoneList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ZoneList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
pub struct Secret {
    #[n(0)]
    pub(crate) name: String,
}

impl Encodable for Secret {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for Secret {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct SecretList(#[n(0)] pub(crate) Vec<Secret>);

impl Encodable for SecretList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for SecretList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct EcrCredentials {
    #[n(1)] pub customer: String,
    #[n(2)] pub image_name: String,
    #[n(3)] pub repository_uri: String,
    #[n(4)] pub auth_token: String,
}

impl Encodable for EcrCredentials {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for EcrCredentials {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
