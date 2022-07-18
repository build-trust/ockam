use minicbor::{Decode, Encode};
use ockam_core::Route;

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

pub mod enroll;
pub mod invitation;
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
    #[b(2)] pub route: CowStr<'a>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new(req: T, route: &Route) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            route: route.to_string().into(),
        }
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(route: &Route) -> Self {
        Self::new((), route)
    }
}
