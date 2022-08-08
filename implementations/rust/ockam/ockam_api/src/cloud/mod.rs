use std::str::FromStr;

use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowStr, Result, Route};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;

pub mod enroll;
pub mod project;
pub mod space;

/// A wrapper around a cloud request with extra fields.
#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CloudRequestWrapper<'a, T> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8956240>,
    #[b(1)] pub req: T,
    #[b(2)] route: CowStr<'a>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new(req: T, route: &MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            route: route.to_string().into(),
        }
    }

    pub fn route(&self) -> Result<Route> {
        let maddr = MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))?;
        crate::multiaddr_to_route(&maddr)
            .ok_or_else(|| ApiError::generic(&format!("Invalid MultiAddr: {}", maddr)))
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(route: &MultiAddr) -> Self {
        Self::new((), route)
    }
}
