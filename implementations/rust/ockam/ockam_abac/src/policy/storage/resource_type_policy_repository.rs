use crate::policy::ResourceTypePolicy;
use crate::{Action, Expr, ResourceType};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

/// This repository stores policies for resources types.
/// A policy is an expression which can be evaluated against an environment (a list of attribute
/// names and values) in order to determine if a given action can be performed on a given resource.
#[async_trait]
pub trait ResourceTypePoliciesRepository: Send + Sync + 'static {
    /// Store a policy for a given resource type and action
    async fn store_policy(
        &self,
        resource_type: &ResourceType,
        action: &Action,
        expression: &Expr,
    ) -> Result<()>;

    /// Return the policy associated to a given resource type and action
    async fn get_policy(
        &self,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<Option<ResourceTypePolicy>>;

    /// Return the list of all the resource type policies
    async fn get_policies(&self) -> Result<Vec<ResourceTypePolicy>>;

    /// Return the list of all the policies associated to a given resource type
    async fn get_policies_by_resource_type(
        &self,
        resource_type: &ResourceType,
    ) -> Result<Vec<ResourceTypePolicy>>;

    /// Delete the policy associated to a given resource type and action
    async fn delete_policy(&self, resource_type: &ResourceType, action: &Action) -> Result<()>;
}
