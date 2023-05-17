use minicbor::{Decode, Encode};
use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpConnection {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1503320>,
    /// The address payload for the transport
    #[b(3)] pub addr: String,
}

impl CreateTcpConnection {
    pub fn new(addr: String) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
        }
    }
}

/// Request body when instructing a node to create a transport
#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateTcpListener {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7461454>,
    /// The address payload for the transport
    #[b(3)] pub addr: String,
}

impl CreateTcpListener {
    pub fn new(addr: String) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
        }
    }
}

/// Request to delete a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteTransport<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4739996>,
    /// The transport ID to delete
    #[b(1)] pub tid: CowStr<'a>,
}

impl<'a> DeleteTransport<'a> {
    pub fn new<S: Into<CowStr<'a>>>(tid: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tid: tid.into(),
        }
    }
}
