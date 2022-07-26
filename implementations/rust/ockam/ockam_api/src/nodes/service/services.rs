use crate::auth::Server;
use crate::identity::IdentityService;
use crate::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartIdentityServiceRequest, StartVaultServiceRequest,
};
use crate::nodes::NodeMan;
use crate::vault::VaultService;
use crate::{Request, Response, ResponseBuilder};
use minicbor::Decoder;
use ockam::{Address, AsyncTryClone, Context, Result};

impl NodeMan {
    pub(super) async fn start_vault_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        let vault = self.vault()?.async_try_clone().await?;
        let service = VaultService::new(vault);

        ctx.start_worker(addr.clone(), service).await?;

        self.registry
            .vault_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_vault_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: StartVaultServiceRequest = dec.decode()?;

        let addr = req_body.addr.to_string().into();

        self.start_vault_service_impl(ctx, addr).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }

    pub(super) async fn start_identity_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        let vault = self.vault()?.async_try_clone().await?;
        IdentityService::create(ctx, addr.clone(), vault).await?;

        self.registry
            .identity_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_identity_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: StartIdentityServiceRequest = dec.decode()?;

        let addr = req_body.addr.to_string().into();

        self.start_identity_service_impl(ctx, addr).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }

    pub(super) async fn start_authenticated_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        let s = self.authenticated_storage.async_try_clone().await?;
        let server = Server::new(s);
        ctx.start_worker(addr.clone(), server).await?;

        self.registry
            .authenticated_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_authenticated_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: StartAuthenticatedServiceRequest = dec.decode()?;

        let addr = req_body.addr.to_string().into();

        self.start_authenticated_service_impl(ctx, addr).await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
