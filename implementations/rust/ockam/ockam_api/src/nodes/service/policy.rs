use ockam_abac::{Action, PolicyExpression};
use ockam_core::api::{Error, Request, Response};
use ockam_core::{async_trait, Result};
use ockam_node::Context;
use std::str::FromStr;

use crate::nodes::models::policies::{PoliciesList, Policy, ResourceTypeOrName, SetPolicyRequest};
use crate::nodes::{BackgroundNodeClient, NodeManagerWorker};

use super::NodeManager;

impl NodeManagerWorker {
    pub(super) async fn add_policy(
        &self,
        action: &str,
        resource: ResourceTypeOrName,
        expression: PolicyExpression,
    ) -> Result<Response<()>, Response<Error>> {
        self.node_manager
            .set_policy(resource, action, expression)
            .await
            .map(|_| Response::ok())
            .map_err(|e| Response::internal_error_no_request(&e.to_string()))
    }

    pub(super) async fn get_policy(
        &self,
        action: &str,
        resource: ResourceTypeOrName,
    ) -> Result<Response<Policy>, Response<Error>> {
        match self.node_manager.get_policy(resource.clone(), action).await {
            Ok(Some(policy)) => Ok(Response::ok().body(policy)),
            Ok(None) => Err(Response::not_found_no_request(&format!(
                "No policy found for resource '{resource}' and action '{action}'"
            ))),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn list_policies(
        &self,
        resource: Option<ResourceTypeOrName>,
    ) -> Result<Response<PoliciesList>, Response<Error>> {
        match self.node_manager.get_policies(resource).await {
            Ok(policies) => Ok(Response::ok().body(policies)),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn delete_policy(
        &self,
        action: &str,
        resource: ResourceTypeOrName,
    ) -> Result<Response<()>, Response<Error>> {
        match self.node_manager.delete_policy(resource, action).await {
            Ok(_) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    /// Set a policy on a resource accessed with a specific action
    pub async fn set_policy(
        &self,
        resource: ResourceTypeOrName,
        action: &str,
        expression: PolicyExpression,
    ) -> Result<()> {
        let action = Action::from_str(action)?;
        match resource {
            ResourceTypeOrName::Type(resource_type) => {
                self.policies()
                    .store_policy_for_resource_type(
                        &resource_type,
                        &action,
                        &expression.to_expression(),
                    )
                    .await
            }
            ResourceTypeOrName::Name(resource_name) => {
                self.policies()
                    .store_policy_for_resource_name(
                        &resource_name,
                        &action,
                        &expression.to_expression(),
                    )
                    .await
            }
        }
    }

    /// Return the policy set on a resource for a given action, if there is one
    pub async fn get_policy(
        &self,
        resource: ResourceTypeOrName,
        action: &str,
    ) -> Result<Option<Policy>> {
        let action = Action::from_str(action)?;
        Ok(match resource {
            ResourceTypeOrName::Type(resource_type) => self
                .policies()
                .get_policy_for_resource_type(&resource_type, &action)
                .await?
                .map(|p| p.into()),
            ResourceTypeOrName::Name(resource_name) => self
                .policies()
                .get_policy_for_resource_name(&resource_name, &action)
                .await?
                .map(|p| p.into()),
        })
    }

    pub async fn get_policies(&self, resource: Option<ResourceTypeOrName>) -> Result<PoliciesList> {
        match resource {
            Some(resource) => match resource {
                ResourceTypeOrName::Type(resource_type) => {
                    let resource_type_policies = self
                        .policies()
                        .get_policies_for_resource_type(&resource_type)
                        .await?;
                    Ok(PoliciesList::new(vec![], resource_type_policies))
                }
                ResourceTypeOrName::Name(resource_name) => {
                    let resource_policies = self
                        .policies()
                        .get_policies_for_resource_name(&resource_name)
                        .await?;
                    Ok(PoliciesList::new(resource_policies, vec![]))
                }
            },
            None => {
                let (resource_policies, resource_type_policies) =
                    self.policies().get_policies().await?;
                Ok(PoliciesList::new(resource_policies, resource_type_policies))
            }
        }
    }

    pub async fn delete_policy(&self, resource: ResourceTypeOrName, action: &str) -> Result<()> {
        let action = Action::from_str(action)?;
        match resource {
            ResourceTypeOrName::Type(resource_type) => {
                self.policies()
                    .delete_policy_for_resource_type(&resource_type, &action)
                    .await
            }
            ResourceTypeOrName::Name(resource_name) => {
                self.policies()
                    .delete_policy_for_resource_name(&resource_name, &action)
                    .await
            }
        }
    }
}

pub fn policy_path(a: &Action) -> String {
    format!("/policy/{a}")
}

#[async_trait]
pub trait Policies {
    async fn add_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
        expression: &PolicyExpression,
    ) -> miette::Result<()>;

    async fn show_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
    ) -> miette::Result<Policy>;

    async fn list_policies(
        &self,
        ctx: &Context,
        resource: Option<&ResourceTypeOrName>,
    ) -> miette::Result<PoliciesList>;

    async fn delete_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
    ) -> miette::Result<()>;
}

#[async_trait]
impl Policies for BackgroundNodeClient {
    async fn add_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
        expression: &PolicyExpression,
    ) -> miette::Result<()> {
        let payload = SetPolicyRequest::new(resource.clone(), expression.clone());
        let request = Request::post(policy_path(action)).body(payload);
        self.tell(ctx, request).await?;
        Ok(())
    }

    async fn show_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
    ) -> miette::Result<Policy> {
        let request = Request::get(policy_path(action)).body(resource);
        self.ask(ctx, request).await
    }

    async fn list_policies(
        &self,
        ctx: &Context,
        resource: Option<&ResourceTypeOrName>,
    ) -> miette::Result<PoliciesList> {
        let request = Request::get("/policy").body(resource);
        self.ask(ctx, request).await
    }

    async fn delete_policy(
        &self,
        ctx: &Context,
        resource: &ResourceTypeOrName,
        action: &Action,
    ) -> miette::Result<()> {
        let request = Request::delete(policy_path(action)).body(resource);
        self.tell(ctx, request).await?;
        Ok(())
    }
}
