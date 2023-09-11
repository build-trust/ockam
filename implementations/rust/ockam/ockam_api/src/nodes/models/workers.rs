use minicbor::{Decode, Encode};

#[derive(Debug, Clone, Decode, Encode)]
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

/// Response body for listing workers
#[derive(Debug, Clone, Decode, Encode)]
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
