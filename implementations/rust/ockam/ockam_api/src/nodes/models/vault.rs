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
    #[n(2)] with_aws: Option<bool>
}

impl<'a> CreateVaultRequest<'a> {
    pub fn new(path: Option<impl Into<CowStr<'a>>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            path: path.map(|p| p.into()),
            with_aws: None
        }
    }

    pub fn with_aws_kms(mut self, val: bool) -> Self {
        self.with_aws = val.then_some(true);
        self
    }

    pub fn is_aws_enabled(&self) -> bool {
        self.with_aws.unwrap_or(false)
    }
}
