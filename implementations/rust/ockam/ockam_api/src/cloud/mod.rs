use minicbor::{Decode, Encode};
use ockam_core::{self, Address};

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
    #[b(2)] pub cloud_address: CowStr<'a>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new(req: T, cloud_node_address: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            cloud_address: cloud_node_address.to_string().into(),
        }
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(cloud_node_address: &Address) -> Self {
        Self::new((), cloud_node_address)
    }
}
