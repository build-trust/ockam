//! Nodemanager API types

use minicbor::{Decode, Encode};
use ockam_core::CowStr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6586555>,
    #[b(1)] pub node_name: CowStr<'a>,
    #[b(2)] pub status: CowStr<'a>,
    #[n(3)] pub workers: u32,
    #[n(4)] pub pid: i32,
}

impl<'a> NodeStatus<'a> {
    pub fn new(
        node_name: impl Into<CowStr<'a>>,
        status: impl Into<CowStr<'a>>,
        workers: u32,
        pid: i32,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            node_name: node_name.into(),
            status: status.into(),
            workers,
            pid,
        }
    }
}
