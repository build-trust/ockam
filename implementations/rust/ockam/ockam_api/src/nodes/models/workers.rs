use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerStatus {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2610323>,
    #[b(2)] pub addr: String,
}

impl WorkerStatus {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Response body for listing workers
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerList {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7336987>,
    #[b(1)] pub list: Vec<WorkerStatus>,
}

impl WorkerList {
    pub fn new(list: Vec<WorkerStatus>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
