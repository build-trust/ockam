use crate::nodes::models::policy::{Policy, PolicyList};
use either::Either;
use minicbor::Decoder;
use ockam_abac::{Action, Resource};
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::Result;

use super::NodeManager;

impl NodeManager {
    pub(super) async fn add_policy(
        &self,
        resource: &str,
        action: &str,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let p: Policy = dec.decode()?;
        let r = Resource::new(resource);
        let a = Action::new(action);
        self.policies.set_policy(&r, &a, p.expression()).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn get_policy<'a>(
        &self,
        req: &'a Request<'_>,
        resource: &str,
        action: &str,
    ) -> Result<Either<ResponseBuilder<Error<'a>>, ResponseBuilder<Policy>>> {
        let r = Resource::new(resource);
        let a = Action::new(action);
        if let Some(e) = self.policies.get_policy(&r, &a).await? {
            Ok(Either::Right(Response::ok(req.id()).body(Policy::new(e))))
        } else {
            let mut err = Error::new(req.path()).with_message("policy not found");
            if let Some(m) = req.method() {
                err.set_method(m)
            }
            Ok(Either::Left(Response::not_found(req.id()).body(err)))
        }
    }

    pub(super) async fn list_policies(
        &self,
        req: &Request<'_>,
        res: &str,
    ) -> Result<ResponseBuilder<PolicyList>> {
        let r = Resource::new(res);
        let p = self.policies.policies(&r).await?;
        Ok(Response::ok(req.id()).body(PolicyList::new(p)))
    }

    pub(super) async fn del_policy(
        &self,
        req: &Request<'_>,
        res: &str,
        act: &str,
    ) -> Result<ResponseBuilder<()>> {
        let r = Resource::new(res);
        let a = Action::new(act);
        self.policies.del_policy(&r, &a).await?;
        Ok(Response::ok(req.id()))
    }
}
