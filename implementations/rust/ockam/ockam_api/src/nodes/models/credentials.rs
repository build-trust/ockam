//! Credentials request/response types

use minicbor::{Decode, Encode};

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
pub struct GetCredentialRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8479533>,
    #[n(1)] pub route: MultiAddr,
    #[n(2)] pub overwrite: bool,
}

impl GetCredentialRequest {
    pub fn new(route: MultiAddr, overwrite: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route,
            overwrite,
        }
    }
}

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PresentCredentialRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3698687>,
    #[n(1)] pub route: MultiAddr,
    #[n(2)] pub oneway: bool,
}

impl PresentCredentialRequest {
    pub fn new(route: MultiAddr, oneway: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route,
            oneway,
        }
    }
}
