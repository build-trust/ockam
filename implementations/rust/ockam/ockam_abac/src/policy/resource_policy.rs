use crate::{Action, Expr, ResourceName};
use minicbor::{Decode, Encode};

#[derive(Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ResourcePolicy {
    #[n(1)] pub resource_name: ResourceName,
    #[n(2)] pub action: Action,
    #[n(3)] pub expression: Expr,
}

impl ResourcePolicy {
    pub fn new(resource_name: ResourceName, action: Action, expression: Expr) -> Self {
        ResourcePolicy {
            resource_name,
            action,
            expression,
        }
    }
}
