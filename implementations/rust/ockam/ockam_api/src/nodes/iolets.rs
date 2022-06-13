//! Inlets and outlet request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body to create an inlet
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInlet<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1407961>,
    #[n(1)] pub addr: Cow<'a, str>,
}

/// Request body to create an outlet
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateOutlet<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1407961>,
    #[n(1)] pub addr: Cow<'a, str>,
}

/// Respons body when interacting with a transport
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct IoletStatus<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    tag: TypeTag<1581592>,
    #[n(1)] pub payload: Cow<'a, str>,
    /// Iolet ID inside the node manager
    ///
    /// We use this as a kind of URI to be able to address a transport
    /// by a unique value for specific updates and deletion events.
    #[n(2)] pub ioid: Cow<'a, str>,
}
