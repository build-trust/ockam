//! Nodemanager API types

// TODO: split up this file into sub modules

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

///////////////////-!  REQUEST BODIES

///////////////////-!  RESPONSE BODIES

/// Response body for a node status
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6586555>,
    #[n(1)] pub node_name: Cow<'a, str>,
    #[n(2)] pub status: Cow<'a, str>,
    #[n(3)] pub workers: u32,
    #[n(4)] pub pid: i32,
    #[n(5)] pub transports: u32,
}

impl<'a> NodeStatus<'a> {
    pub fn new(
        node_name: &'a str,
        status: &'a str,
        workers: u32,
        pid: i32,
        transports: u32,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            node_name: Cow::Borrowed(node_name),
            status: Cow::Borrowed(status),
            workers,
            pid,
            transports,
        }
    }
}
