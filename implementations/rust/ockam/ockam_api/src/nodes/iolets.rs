//! Inlets and outlet request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Distinguish between inlet and outlet portals in the API
#[derive(Copy, Clone, Debug, Decode, Encode, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum IoletType {
    #[n(0)] Inlet,
    #[n(1)] Outlet,
}

/// Request body to create an inlet or outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateIolet<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1407961>,
    /// The type of portal endpoint to create
    #[n(1)] pub tt: IoletType,
    /// The address the portal should connect or bind to
    #[b(2)] pub addr: Cow<'a, str>,
    /// The forwarding address (must be ockam routing address)
    ///
    /// This field will be disregarded for portal outlets.  Portal
    /// inlets MUST set this value to configure their forwarding
    /// behaviour.  This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[b(3)] pub fwd: Option<Cow<'a, str>>,
    /// A human-friendly alias for this portal endpoint
    #[b(4)] pub alias: Option<Cow<'a, str>>,
}

impl<'a> CreateIolet<'a> {
    pub fn new(
        tt: IoletType,
        addr: impl Into<Cow<'a, str>>,
        fwd: impl Into<Option<Cow<'a, str>>>,
        alias: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            addr: addr.into(),
            fwd: fwd.into(),
            alias: alias.into(),
        }
    }
}

/// Respons body when interacting with an inlet or outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct IoletStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1581592>,
    #[n(1)] pub tt: IoletType,
    #[b(2)] pub addr: Cow<'a, str>,
    #[b(3)] pub alias: Cow<'a, str>,
    /// An optional status payload
    #[b(4)] pub payload: Option<Cow<'a, str>>,
}

impl<'a> IoletStatus<'a> {
    pub fn new(
        tt: IoletType,
        addr: impl Into<Cow<'a, str>>,
        alias: impl Into<Cow<'a, str>>,
        payload: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            addr: addr.into(),
            alias: alias.into(),
            payload: payload.into(),
        }
    }
}

/// Responsebody when returning a list of Iolets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct IoletList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<8401504>,
    #[b(1)] pub list: Vec<IoletStatus<'a>>
}

impl<'a> IoletList<'a> {
    pub fn new(list: Vec<IoletStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
