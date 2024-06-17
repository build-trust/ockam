use crate::colors::OckamColor;
use crate::output::Output;
use crate::Result;
use colorful::Colorful;
use minicbor::{CborLen, Decode, Encode};

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerStatus {
    #[n(2)] pub addr: String,
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
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerList {
    #[n(1)] pub list: Vec<WorkerStatus>,
}

impl WorkerList {
    pub fn new(list: Vec<WorkerStatus>) -> Self {
        Self { list }
    }
}
