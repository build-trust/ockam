use either::Either;
use minicbor::Decoder;

use ockam_abac::{Action, Resource};
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::Result;

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
