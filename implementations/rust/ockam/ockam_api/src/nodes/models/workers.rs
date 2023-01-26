use minicbor::{Decode, Encode};
use ockam_core::CowStr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct WorkerStatus<'a>  {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2610323>,
    #[b(2)] pub addr: CowStr<'a>,
}

impl<'a> WorkerStatus<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct WorkerList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7336987>,
    #[b(1)] pub list: Vec<WorkerStatus<'a>>
}

impl<'a> WorkerList<'a> {
    pub fn new(list: Vec<WorkerStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
