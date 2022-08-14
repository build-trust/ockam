//! Inlets and outlet request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body to create an inlet or outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInlet<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1407961>,
    /// The address the portal should bind to
    #[b(1)] pub bind_addr: Cow<'a, str>,
    /// The peer address (must be ockam routing address)
    /// This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[b(2)] pub outlet_route: Cow<'a, str>,
    /// A human-friendly alias for this portal endpoint
    #[b(3)] pub alias: Option<CowStr<'a>>,
}

impl<'a> CreateInlet<'a> {
    pub fn new(
        bind_addr: impl Into<Cow<'a, str>>,
        outlet_route: impl Into<Cow<'a, str>>,
        alias: impl Into<Option<CowStr<'a>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            bind_addr: bind_addr.into(),
            outlet_route: outlet_route.into(),
            alias: alias.into(),
        }
    }
}

/// Request body to create an inlet or outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateOutlet<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5351558>,
    /// The address the portal should connect or bind to
    #[b(1)] pub tcp_addr: Cow<'a, str>,
    /// The address the portal should connect or bind to
    #[b(2)] pub worker_addr: Cow<'a, str>,
    /// A human-friendly alias for this portal endpoint
    #[b(3)] pub alias: Option<CowStr<'a>>,
}

impl<'a> CreateOutlet<'a> {
    pub fn new(
        tcp_addr: impl Into<Cow<'a, str>>,
        worker_addr: impl Into<Cow<'a, str>>,
        alias: impl Into<Option<CowStr<'a>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tcp_addr: tcp_addr.into(),
            worker_addr: worker_addr.into(),
            alias: alias.into(),
        }
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1581592>,
    #[b(1)] pub bind_addr: Cow<'a, str>,
    #[b(2)] pub worker_addr: Cow<'a, str>,
    #[b(3)] pub alias: Cow<'a, str>,
    /// An optional status payload
    #[b(4)] pub payload: Option<Cow<'a, str>>,
}

impl<'a> InletStatus<'a> {
    pub fn bad_request(reason: &'static str) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            bind_addr: "".into(),
            worker_addr: "".into(),
            alias: "".into(),
            payload: Some(reason.into()),
        }
    }

    pub fn new(
        bind_addr: impl Into<Cow<'a, str>>,
        worker_addr: impl Into<Cow<'a, str>>,
        alias: impl Into<Cow<'a, str>>,
        payload: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            bind_addr: bind_addr.into(),
            worker_addr: worker_addr.into(),
            alias: alias.into(),
            payload: payload.into(),
        }
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4012569>,
    #[b(1)] pub tcp_addr: Cow<'a, str>,
    #[b(2)] pub worker_addr: Cow<'a, str>,
    #[b(3)] pub alias: Cow<'a, str>,
    /// An optional status payload
    #[b(4)] pub payload: Option<Cow<'a, str>>,
}

impl<'a> OutletStatus<'a> {
    pub fn bad_request(reason: &'static str) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tcp_addr: "".into(),
            worker_addr: "".into(),
            alias: "".into(),
            payload: Some(reason.into()),
        }
    }

    pub fn new(
        tcp_addr: impl Into<Cow<'a, str>>,
        worker_addr: impl Into<Cow<'a, str>>,
        alias: impl Into<Cow<'a, str>>,
        payload: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tcp_addr: tcp_addr.into(),
            worker_addr: worker_addr.into(),
            alias: alias.into(),
            payload: payload.into(),
        }
    }
}

/// Response body when returning a list of Inlets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8401504>,
    #[b(1)] pub list: Vec<InletStatus<'a>>
}

impl<'a> InletList<'a> {
    pub fn new(list: Vec<InletStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}

/// Response body when returning a list of Outlets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletList<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8708916>,
    #[b(1)] pub list: Vec<OutletStatus<'a>>
}

impl<'a> OutletList<'a> {
    pub fn new(list: Vec<OutletStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
