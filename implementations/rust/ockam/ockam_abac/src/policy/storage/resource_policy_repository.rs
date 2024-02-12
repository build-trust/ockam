use crate::{Action, Expr, ResourceName, ResourcePolicy};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

/// This repository stores policies for resources.
/// A policy is an expression which can be evaluated against an environment (a list of attribute
/// names and values) in order to determine if a given action can be performed on a given resource.
#[async_trait]
pub trait ResourcePoliciesRepository: Send + Sync + 'static {
    /// Store a policy for a given resource and action
    async fn store_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
        expression: &Expr,
    ) -> Result<()>;

    /// Return the policy associated to a given resource and action
    async fn get_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
    ) -> Result<Option<ResourcePolicy>>;

    /// Return the list of all the resource policies
    async fn get_policies(&self) -> Result<Vec<ResourcePolicy>>;

    /// Return the list of all the policies associated to a given resource name
    async fn get_policies_by_resource_name(
        &self,
        resource_name: &ResourceName,
    ) -> Result<Vec<ResourcePolicy>>;

    /// Delete the policy associated to a given resource name and action
    async fn delete_policy(&self, resource_name: &ResourceName, action: &Action) -> Result<()>;
}
