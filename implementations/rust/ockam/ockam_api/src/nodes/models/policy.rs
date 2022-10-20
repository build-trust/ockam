use minicbor::{Decode, Encode};
use ockam_abac::Expr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Policy {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2000111>,
    #[n(1)] expression: Expr,
}

impl Policy {
    pub fn new(e: Expr) -> Self {
        Policy {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            expression: e,
        }
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }
}
