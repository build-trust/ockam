//! Credentials request/response types

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::CowStr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SetAuthorityRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1864428>,
    #[b(1)] pub authorities: Vec<Vec<u8>>,
}

impl SetAuthorityRequest {
    pub fn new(authorities: Vec<Vec<u8>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            authorities,
        }
    }
}

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetCredentialRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8479533>,
    #[b(1)] pub route: CowStr<'a>,
    #[n(2)] pub overwrite: bool,
}

impl<'a> GetCredentialRequest<'a> {
    pub fn new(route: &MultiAddr, overwrite: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route: route.to_string().into(),
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
