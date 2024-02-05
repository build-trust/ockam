use crate::{Action, Expr, Resource};
use core::fmt::{Display, Formatter};
use minicbor::{Decode, Encode};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

/// This repository stores policies.
/// A policy is an expression which can be evaluated against an environment (a list of attribute
/// names and values) in order to determine if a given action can be performed on a given resource.
#[async_trait]
pub trait PoliciesRepository: Send + Sync + 'static {
    /// Return the policy associated to a given resource and action
    async fn get_policy(&self, resource: &Resource, action: &Action) -> Result<Option<Policy>>;

    /// Set a policy for a given resource and action
    async fn set_policy(&self, resource: &Resource, action: &Action, policy: &Policy)
        -> Result<()>;

    /// Delete the policy associated to a given resource and action
    async fn delete_policy(&self, resource: &Resource, action: &Action) -> Result<()>;

    /// Return the list of all the policies associated to a given resource
    async fn get_policies_by_resource(&self, resource: &Resource) -> Result<Vec<(Action, Policy)>>;
}

#[derive(Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Policy {
    #[n(1)] expression: Expr,
}

impl Policy {
    pub fn new(e: Expr) -> Self {
        Policy { expression: e }
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }
}

impl Display for Policy {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.expression.fmt(f)
    }
}
