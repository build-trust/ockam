//! Inlets and outlet request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Distinguish between inlet and outlet portals in the API
#[derive(Copy, Clone, Debug, Decode, Encode)]
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
    #[n(1)] pub tt: IoletType,
    #[n(2)] pub addr: Cow<'a, str>,
    #[n(3)] pub alias: Option<Cow<'a, str>>,
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
    #[n(2)] pub addr: Cow<'a, str>,
    #[n(3)] pub alias: Cow<'a, str>,
}

impl<'a> IoletStatus<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(tt: IoletType, addr: S, alias: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tt,
            addr: addr.into(),
            alias: alias.into(),
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
    #[n(1)] pub list: Vec<IoletStatus<'a>>
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
