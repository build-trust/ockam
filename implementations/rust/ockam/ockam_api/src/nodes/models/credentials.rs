//! Credentials request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetCredentialRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8479533>,
    #[n(1)] pub overwrite: bool,
}

impl GetCredentialRequest {
    pub fn new(overwrite: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            overwrite,
        }
    }
}

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PresentCredentialRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3698687>,
    #[b(1)] pub route: Cow<'a, str>,
    #[n(2)] pub oneway: bool,
}

impl<'a> PresentCredentialRequest<'a> {
    pub fn new(route: &MultiAddr, oneway: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route: route.to_string().into(),
            oneway,
        }
    }
}
