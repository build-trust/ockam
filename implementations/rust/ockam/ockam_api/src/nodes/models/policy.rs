use minicbor::{Decode, Encode};
use ockam_abac::{Action, Expr};

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

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PolicyList {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3521457>,
    #[n(1)] expressions: Vec<(Action, Expr)>,
}

impl PolicyList {
    pub fn new(e: Vec<(Action, Expr)>) -> Self {
        PolicyList {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            expressions: e,
        }
    }

    pub fn expressions(&self) -> &[(Action, Expr)] {
        &self.expressions
    }
}
