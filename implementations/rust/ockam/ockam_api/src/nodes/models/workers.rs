use crate::colors::OckamColor;
use crate::output::Output;
use crate::Result;
use colorful::Colorful;
use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};

#[derive(Debug, Clone, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerStatus {
    #[n(2)] pub addr: String,
}

impl Encodable for WorkerStatus {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for WorkerStatus {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl WorkerStatus {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

impl Output for WorkerStatus {
    fn item(&self) -> Result<String> {
        Ok(format!(
            "Worker {}",
            self.addr
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))
    }
}

/// Response body for listing workers
#[derive(Debug, Clone, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerList {
    #[n(1)] pub list: Vec<WorkerStatus>,
}

impl Encodable for WorkerList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for WorkerList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl WorkerList {
    pub fn new(list: Vec<WorkerStatus>) -> Self {
        Self { list }
    }
}
