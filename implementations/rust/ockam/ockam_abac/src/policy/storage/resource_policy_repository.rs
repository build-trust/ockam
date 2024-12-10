use crate::{Action, Expr, ResourceName, ResourcePolicy};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

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

#[cfg(feature = "std")]
#[async_trait]
impl<T: ResourcePoliciesRepository> ResourcePoliciesRepository for AutoRetry<T> {
    async fn store_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
        expression: &Expr,
    ) -> Result<()> {
        retry!(self.wrapped.store_policy(resource_name, action, expression))
    }

    async fn get_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
    ) -> Result<Option<ResourcePolicy>> {
        retry!(self.wrapped.get_policy(resource_name, action))
    }

    async fn get_policies(&self) -> Result<Vec<ResourcePolicy>> {
        retry!(self.wrapped.get_policies())
    }

    async fn get_policies_by_resource_name(
        &self,
        resource_name: &ResourceName,
    ) -> Result<Vec<ResourcePolicy>> {
        retry!(self.wrapped.get_policies_by_resource_name(resource_name))
    }

    async fn delete_policy(&self, resource_name: &ResourceName, action: &Action) -> Result<()> {
        retry!(self.wrapped.delete_policy(resource_name, action))
    }
}
