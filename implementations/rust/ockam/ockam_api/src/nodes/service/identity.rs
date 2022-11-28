use super::NodeManagerWorker;
use crate::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};

impl NodeManagerWorker {
    pub(super) async fn long_identity(
        &mut self,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<LongIdentityResponse<'_>>> {
        let node_manager = self.node_manager.read().await;
        let identity = node_manager.identity()?;
        let identity = identity.export().await?;

        let response = Response::ok(req.id()).body(LongIdentityResponse::new(identity));
        Ok(response)
    }

    pub(super) async fn short_identity(
        &mut self,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<ShortIdentityResponse<'_>>> {
        let node_manager = self.node_manager.read().await;
        let identity = node_manager.identity()?;
        let identifier = identity.identifier();

        let response =
            Response::ok(req.id()).body(ShortIdentityResponse::new(identifier.to_string()));
        Ok(response)
    }
}
