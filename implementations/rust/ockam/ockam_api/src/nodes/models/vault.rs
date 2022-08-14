use minicbor::{Decode, Encode};

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body when instructing a node to create a Vault
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateVaultRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8008758>,
    #[b(1)] pub path: Option<CowStr<'a>>,
}

impl<'a> CreateVaultRequest<'a> {
    pub fn new(path: Option<impl Into<CowStr<'a>>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            path: path.map(|p| p.into()),
        }
    }
}
