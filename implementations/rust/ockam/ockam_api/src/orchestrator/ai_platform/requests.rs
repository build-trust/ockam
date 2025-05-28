use std::collections::BTreeMap;

use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::compat::collections::HashMap;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateZone {
    #[n(1)] pub name: String,
}

impl Encodable for CreateZone {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateZone {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateZone {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct ListZones {
    #[n(1)] pub cluster: String,
}

impl Encodable for ListZones {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ListZones {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl ListZones {
    pub fn new(cluster: String) -> Self {
        Self { cluster }
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeployZone {
    #[n(1)] pub manifest: String,
}

impl Encodable for DeployZone {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for DeployZone {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl DeployZone {
    pub fn new(manifest: &serde_json::Value) -> miette::Result<Self> {
        Ok(Self {
            manifest: serde_json::to_string(manifest).into_diagnostic()?,
        })
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecret {
    #[n(1)] pub name: String,
    #[n(2)] pub fields: String,
}

impl Encodable for CreateSecret {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateSecret {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateSecret {
    pub fn new(name: &str, fields: &HashMap<String, String>) -> miette::Result<Self> {
        Ok(Self {
            name: name.to_string(),
            fields: serde_json::to_string(fields).into_diagnostic()?,
        })
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecret {
    #[n(1)] pub name: String,
}

impl Encodable for DeleteSecret {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for DeleteSecret {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl DeleteSecret {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct ProvisionEcr {
    #[n(1)] pub image_names: Vec<String>,
    #[n(2)] pub is_public: bool,
}

impl Encodable for ProvisionEcr {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for ProvisionEcr {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl ProvisionEcr {
    pub fn new(image_names: Vec<String>, is_public: bool) -> Self {
        Self {
            image_names,
            is_public,
        }
    }
}

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateEnrollmentToken {
    #[n(1)] pub attributes: String,
    #[n(2)] pub relay: Option<String>,
}

impl Encodable for CreateEnrollmentToken {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateEnrollmentToken {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateEnrollmentToken {
    pub fn new(
        attributes: BTreeMap<String, String>,
        relay: Option<String>,
    ) -> miette::Result<Self> {
        Ok(Self {
            attributes: serde_json::to_string(&attributes).into_diagnostic()?,
            relay,
        })
    }
}
