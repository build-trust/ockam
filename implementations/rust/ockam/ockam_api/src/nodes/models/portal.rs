//! Inlets and outlet request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Distinguish between inlet and outlet portals in the API
#[derive(Copy, Clone, Debug, Decode, Encode, PartialEq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum PortalType {
    #[n(0)] Inlet,
    #[n(1)] Outlet,
}

/// Request body to create an inlet or outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreatePortal<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1407961>,
    /// The type of portal endpoint to create
    #[n(1)] pub tt: PortalType,
    /// The address the portal should connect or bind to
    #[b(2)] pub addr: Cow<'a, str>,
    /// The peer address (must be ockam routing address)
    ///
    /// This field will be disregarded for portal outlets.  Portal
    /// inlets MUST set this value to configure their forwarding
    /// behaviour.  This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[b(3)] pub peer: Option<Cow<'a, str>>,
    /// A human-friendly alias for this portal endpoint
    #[b(4)] pub alias: Option<Cow<'a, str>>,
}

impl<'a> CreatePortal<'a> {
    pub fn new(
        tt: PortalType,
        addr: impl Into<Cow<'a, str>>,
        peer: impl Into<Option<Cow<'a, str>>>,
        alias: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            addr: addr.into(),
            peer: peer.into(),
            alias: alias.into(),
        }
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PortalStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1581592>,
    #[n(1)] pub tt: PortalType,
    #[b(2)] pub addr: Cow<'a, str>,
    #[b(3)] pub alias: Cow<'a, str>,
    /// An optional status payload
    #[b(4)] pub payload: Option<Cow<'a, str>>,
}

impl<'a> PortalStatus<'a> {
    pub fn bad_request(tt: PortalType, reason: &'static str) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            addr: "".into(),
            alias: "".into(),
            payload: Some(reason.into()),
        }
    }

    pub fn new(
        tt: PortalType,
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

/// Responsebody when returning a list of Portals
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PortalList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<8401504>,
    #[b(1)] pub list: Vec<PortalStatus<'a>>
}

impl<'a> PortalList<'a> {
    pub fn new(list: Vec<PortalStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
