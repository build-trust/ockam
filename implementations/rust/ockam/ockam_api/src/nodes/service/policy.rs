use either::Either;
use minicbor::Decoder;

use crate::cli_state::{StateDirTrait, StateItemTrait};
use crate::nodes::BackgroundNode;
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Resource};
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::{async_trait, Result};
use ockam_node::Context;

use crate::nodes::models::policy::{Expression, Policy, PolicyList};

use super::NodeManager;

impl NodeManager {
    pub(super) async fn add_policy(
        &self,
        resource: &str,
        action: &str,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response<()>, Response<Error>> {
        let p: Policy = dec.decode()?;
        let r = Resource::new(resource);
        let a = Action::new(action);
        self.policies.set_policy(&r, &a, p.expression()).await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn get_policy<'a>(
        &self,
        req: &'a RequestHeader,
        resource: &str,
        action: &str,
    ) -> Result<Either<Response<Error>, Response<Policy>>> {
        let r = Resource::new(resource);
        let a = Action::new(action);
        if let Some(e) = self.policies.get_policy(&r, &a).await? {
            Ok(Either::Right(Response::ok(req).body(Policy::new(e))))
        } else {
            Ok(Either::Left(Response::not_found(req, "policy not found")))
        }
    }

    pub(super) async fn list_policies(
        &self,
        req: &RequestHeader,
        res: &str,
    ) -> Result<Response<PolicyList>, Response<Error>> {
        let r = Resource::new(res);
        let p = self.policies.policies(&r).await?;
        let p = p.into_iter().map(|(a, e)| Expression::new(a, e)).collect();
        Ok(Response::ok(req).body(PolicyList::new(p)))
    }

    pub(super) async fn del_policy(
        &self,
        req: &RequestHeader,
        res: &str,
        act: &str,
    ) -> Result<Response<()>, Response<Error>> {
        let r = Resource::new(res);
        let a = Action::new(act);
        self.policies.del_policy(&r, &a).await?;
        Ok(Response::ok(req))
    }
}

pub(crate) fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}

#[async_trait]
pub trait Policies {
    async fn add_policy_to_project(&self, ctx: &Context, resource_name: &str)
        -> miette::Result<()>;
}

#[async_trait]
impl Policies for BackgroundNode {
    async fn add_policy_to_project(
        &self,
        ctx: &Context,
        resource_name: &str,
    ) -> miette::Result<()> {
        let project_id = match self
            .cli_state()
            .nodes
            .get(self.node_name())?
            .config()
            .setup()
            .project
            .to_owned()
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
}
