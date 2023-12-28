use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Policy, Resource};
use ockam_core::api::{Error, Request, Response};
use ockam_core::{async_trait, Result};
use ockam_node::Context;

use crate::nodes::models::policy::{Expression, PolicyList};
use crate::nodes::{BackgroundNodeClient, NodeManagerWorker};

use super::NodeManager;

impl NodeManagerWorker {
    pub(super) async fn add_policy(
        &self,
        resource: &str,
        action: &str,
        policy: Policy,
    ) -> Result<Response<()>, Response<Error>> {
        let resource = Resource::new(resource);
        let action = Action::new(action);
        self.node_manager
            .set_policy(resource, action, policy)
            .await
            .map(|_| Response::ok())
            .map_err(|e| Response::internal_error_no_request(&e.to_string()))
    }

    pub(super) async fn get_policy(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Response<Policy>, Response<Error>> {
        let resource = Resource::new(resource);
        let action = Action::new(action);
        match self
            .node_manager
            .get_policy(resource.clone(), action.clone())
            .await
        {
            Ok(Some(policy)) => Ok(Response::ok().body(policy)),
            Ok(None) => Err(Response::not_found_no_request(&format!(
                "no policy found for {resource}/{action}"
            ))),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn list_policies(
        &self,
        resource: &str,
    ) -> Result<Response<PolicyList>, Response<Error>> {
        let resource = Resource::new(resource);
        match self.node_manager.get_policies_by_resource(&resource).await {
            Ok(policies) => Ok(Response::ok().body(PolicyList::new(
                policies
                    .into_iter()
                    .map(|(a, p)| Expression::new(a, p.expression().clone()))
                    .collect(),
            ))),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn delete_policy(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Response<()>, Response<Error>> {
        let resource = Resource::new(resource);
        let action = Action::new(action);
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
        resource: Resource,
        action: Action,
        policy: Policy,
    ) -> Result<()> {
        Ok(self
            .cli_state
            .set_policy(&resource, &action, &policy)
            .await?)
    }

    /// Return the policy set on a resource for a given action, if there is one
    pub async fn get_policy(&self, resource: Resource, action: Action) -> Result<Option<Policy>> {
        Ok(self.cli_state.get_policy(&resource, &action).await?)
    }

    pub async fn get_policies_by_resource(
        &self,
        resource: &Resource,
    ) -> Result<Vec<(Action, Policy)>> {
        Ok(self.cli_state.get_policies_by_resource(resource).await?)
    }

    pub async fn delete_policy(&self, resource: Resource, action: Action) -> Result<()> {
        Ok(self.cli_state.delete_policy(&resource, &action).await?)
    }
}

pub(crate) fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}

#[async_trait]
pub trait Policies {
    async fn add_policy_to_project(&self, ctx: &Context, resource_name: &str)
        -> miette::Result<()>;

    async fn add_policy(
        &self,
        ctx: &Context,
        resource_name: &Resource,
        action: &Action,
        policy: &Policy,
    ) -> miette::Result<()>;
}

#[async_trait]
impl Policies for BackgroundNodeClient {
    async fn add_policy_to_project(
        &self,
        ctx: &Context,
        resource_name: &str,
    ) -> miette::Result<()> {
        let project_id = match self
            .cli_state()
            .get_node_project(&self.node_name())
            .await
            .ok()
        {
            None => return Ok(()),
            Some(p) => p.id,
        };

        let resource = Resource::new(resource_name);
        let policies: PolicyList = self
            .ask(ctx, Request::get(format!("/policy/{resource}")))
            .await?;
        if !policies.expressions().is_empty() {
            return Ok(());
        }

        let policy = {
            let expr = eq([ident("subject.trust_context_id"), str(project_id)]);
            Policy::new(expr)
        };
        let action = Action::new("handle_message");
        let request = Request::post(policy_path(&resource, &action)).body(policy);
        self.tell(ctx, request).await?;

        Ok(())
    }

    async fn add_policy(
        &self,
        ctx: &Context,
        resource: &Resource,
        action: &Action,
        policy: &Policy,
    ) -> miette::Result<()> {
        let request = Request::post(policy_path(resource, action)).body(policy);
        self.tell(ctx, request).await?;

        Ok(())
    }
}
