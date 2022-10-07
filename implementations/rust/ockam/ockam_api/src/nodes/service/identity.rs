use super::{map_anyhow_err, NodeManagerWorker};
use crate::nodes::models::identity::{
    CreateIdentityResponse, LongIdentityResponse, ShortIdentityResponse,
};
use crate::nodes::NodeManager;
use ockam::identity::{Identity, IdentityIdentifier};
use ockam::{Context, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::errcode::{Kind, Origin};

impl NodeManager {
    pub(super) async fn create_identity_impl(
        &mut self,
        ctx: &Context,
        reuse_if_exists: bool,
    ) -> Result<IdentityIdentifier> {
        if let Some(identity) = &self.identity {
            return if reuse_if_exists {
                debug!("Using existing identity");
                Ok(identity.identifier().clone())
            } else {
                Err(ockam_core::Error::new(
                    Origin::Application,
                    Kind::AlreadyExists,
                    "Identity already exists",
                ))
            };
        }

        let vault = self.vault()?;

        let identity = Identity::create(ctx, vault).await?;
        let identifier = identity.identifier().clone();
        let exported_identity = identity.export().await?;

        let state = self.config.state();
        state.write().identity = Some(exported_identity);
        state.persist_config_updates().map_err(map_anyhow_err)?;

        self.identity = Some(identity);

        Ok(identifier)
    }
}

impl NodeManagerWorker {
    pub(super) async fn create_identity(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<CreateIdentityResponse<'_>>> {
        let mut node_manager = self.node_manager.write().await;
        let identifier = node_manager.create_identity_impl(ctx, false).await?;

        let response =
            Response::ok(req.id()).body(CreateIdentityResponse::new(identifier.to_string()));
        Ok(response)
    }

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
