use ockam::identity::{Identity, IdentityIdentifier};
use ockam::{Context, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::compat::collections::HashMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_vault::Vault;

use crate::nodes::models::identity::{
    CreateIdentityResponse, LongIdentityResponse, ShortIdentityResponse,
};
use crate::nodes::NodeManager;

use super::{map_anyhow_err, NodeManagerWorker};

impl NodeManager {
    pub(super) async fn create_identity_impl(
        &mut self,
        ctx: &Context,
        reuse_if_exists: bool,
    ) -> Result<IdentityIdentifier> {
        if reuse_if_exists && self.default_identity.is_some() {
            return self.default_identity.ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Identity,
                    Kind::NotFound,
                    "default identity doesn't exist",
                )
            });
        }

        let vault = self.vault()?;

        let identity = Identity::create(ctx, vault).await?;
        let identifier = identity.identifier().clone();

        self.config.state().write().add_identity(&identity)?;

        state.persist_config_updates().map_err(map_anyhow_err)?;

        self.add_identity(identifier.clone(), identity);

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
