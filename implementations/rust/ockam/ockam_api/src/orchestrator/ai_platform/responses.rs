use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Cluster (
    #[n(1)] pub String,
);

impl Encodable for Cluster {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for Cluster {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl Cluster {
    pub fn new(cluster: String) -> Self {
        Self(cluster)
    }

    pub fn into_inner(self) -> String {
        self.0.to_string()
    }
}

impl Display for Cluster {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
pub struct ZoneNameList {
    #[n(0)]
    pub(crate) zones: Vec<String>,
}

impl Encodable for ZoneNameList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ZoneNameList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct ZoneList {
    #[n(0)]
    pub(crate) zones: Vec<Zone>,
}

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
#[cbor(map)]
pub struct Secret {
    #[n(1)]
    #[serde(alias = "secret")]
    pub name: String,
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
pub struct SecretNameList {
    #[n(0)]
    pub(crate) secrets: Vec<String>,
}

impl Encodable for SecretNameList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for SecretNameList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct SecretList {
    #[n(0)]
    pub(crate) secrets: Vec<Secret>,
}

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
pub struct EcrCredential {
    #[serde(alias="customer")]
    #[n(1)] pub cluster: String,
    #[n(2)] pub images: BTreeMap<String, String>, // pairs of (image name, ECR URI)
    #[n(4)] pub auth_token: String,
}

impl Encodable for EcrCredential {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for EcrCredential {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Ticket {
    #[serde(alias="token")]
    #[n(1)] pub ticket: String,
}

impl Encodable for Ticket {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for Ticket {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
