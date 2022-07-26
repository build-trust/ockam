use super::map_anyhow_err;
use crate::error::ApiError;
use crate::nodes::models::identity::{
    CreateIdentityResponse, ExportIdentityResponse, PrintIdentityResponse,
};
use crate::nodes::NodeMan;
use crate::{Request, Response, ResponseBuilder};
use ockam::identity::{Identity, IdentityIdentifier};
use ockam::vault::Vault;
use ockam::{Context, Result};

impl NodeMan {
    pub(crate) fn identity(&self) -> Result<&Identity<Vault>> {
        self.identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))
    }

    pub(super) async fn create_identity_impl(
        &mut self,
        ctx: &Context,
    ) -> Result<IdentityIdentifier> {
        if self.identity.is_some() {
            return Err(ApiError::generic("Identity already exists"))?;
        }

        let vault = self.vault()?;

        let identity = Identity::create(ctx, vault).await?;
        let identifier = identity.identifier()?;
        let exported_identity = identity.export().await?;

        self.config.inner().write().unwrap().identity = Some(exported_identity);
        self.config.atomic_update().run().map_err(map_anyhow_err)?;

        self.identity = Some(identity);

        Ok(identifier)
    }

    pub(super) async fn create_identity(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<CreateIdentityResponse<'_>>> {
        let identifier = self.create_identity_impl(ctx).await?;

        let response =
            Response::ok(req.id()).body(CreateIdentityResponse::new(identifier.to_string()));
        Ok(response)
    }

    pub(super) async fn export_identity(
        &mut self,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<ExportIdentityResponse<'_>>> {
        let identity = self.identity()?;
        let identity = identity.export().await?;

        let response = Response::ok(req.id()).body(ExportIdentityResponse::new(identity));
        Ok(response)
    }

    pub(super) async fn print_identity(
        &mut self,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<PrintIdentityResponse<'_>>> {
        let identity = self.identity()?;
        let identifier = identity.identifier()?;

        let response =
            Response::ok(req.id()).body(PrintIdentityResponse::new(identifier.to_string()));
        Ok(response)
    }
}
