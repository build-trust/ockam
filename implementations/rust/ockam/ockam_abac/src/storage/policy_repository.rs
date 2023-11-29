use crate::{Action, Expr, Resource};
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
    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Expr>>;

    /// Set a policy for a given resource and action
    async fn set_policy(&self, r: &Resource, a: &Action, c: &Expr) -> Result<()>;

    /// Delete the policy associated to a given resource and action
    async fn delete_policy(&self, r: &Resource, a: &Action) -> Result<()>;

    /// Return the list of all the policies associated to a given resource
    async fn get_policies_by_resource(&self, r: &Resource) -> Result<Vec<(Action, Expr)>>;
}
