use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::identity::IdentityService;
use crate::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartCredentialsService,
    StartEchoerServiceRequest, StartIdentityServiceRequest, StartUppercaseServiceRequest,
    StartVaultServiceRequest, StartVerifierService,
};
use crate::nodes::registry::{CredentialsServiceInfo, VerifierServiceInfo};
use crate::nodes::NodeManager;
use crate::uppercase::Uppercase;
use crate::vault::VaultService;
use minicbor::Decoder;
use ockam::{Address, AsyncTryClone, Context, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};

impl NodeManager {
    pub(super) async fn start_vault_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.vault_services.contains_key(&addr) {
            return Err(ApiError::generic("Vault service at this address exists"));
        }

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

        let response = match self.start_vault_service_impl(ctx, addr).await {
            Ok(_) => Response::ok(req.id()),
            Err(_err) => Response::bad_request(req.id()),
        };

        Ok(response)
    }

    pub(super) async fn start_identity_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.identity_services.contains_key(&addr) {
            return Err(ApiError::generic("Identity service at this address exists"));
        }

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

        let response = match self.start_identity_service_impl(ctx, addr).await {
            Ok(_) => Response::ok(req.id()),
            Err(_err) => Response::bad_request(req.id()),
        };

        Ok(response)
    }

    pub(super) async fn start_verifier_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let body: StartVerifierService = dec.decode()?;
        let addr: Address = body.address().into();

        if self.registry.verifier_services.contains_key(&addr) {
            return Err(ApiError::generic("verifier exists at this address"));
        }

        let vault = if let Some(v) = &self.vault {
            v.async_try_clone().await?
        } else {
            return Err(ApiError::generic("vault not found"));
        };

        let vs = crate::verifier::Verifier::new(vault);
        ctx.start_worker(addr.clone(), vs).await?;

        self.registry
            .verifier_services
            .insert(addr, VerifierServiceInfo::default());

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_credentials_service_impl<'a>(
        &mut self,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "credentials service exists at this address",
            ));
        }

        let identity = self.identity()?;

        let authorities = self.authorities()?;

        identity
            .start_credentials_exchange_worker(
                authorities.clone(),
                addr.clone(),
                !oneway,
                self.authenticated_storage.async_try_clone().await?,
            )
            .await?;

        self.registry
            .credentials_services
            .insert(addr, CredentialsServiceInfo::default());

        Ok(())
    }

    pub(super) async fn start_credentials_service<'a>(
        &mut self,
        _ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();

        self.start_credentials_service_impl(addr, oneway).await?;

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_authenticated_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.authenticated_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Authenticated service at this address exists",
            ));
        }

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

        let response = match self.start_authenticated_service_impl(ctx, addr).await {
            Ok(_) => Response::ok(req.id()),
            Err(_err) => Response::bad_request(req.id()),
        };

        Ok(response)
    }

    pub(super) async fn start_uppercase_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Uppercase service at this address exists",
            ));
        }

        ctx.start_worker(addr.clone(), Uppercase).await?;

        self.registry
            .uppercase_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_uppercase_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: StartUppercaseServiceRequest = dec.decode()?;

        let addr = req_body.addr.to_string().into();

        let response = match self.start_uppercase_service_impl(ctx, addr).await {
            Ok(_) => Response::ok(req.id()),
            Err(_err) => Response::bad_request(req.id()),
        };

        Ok(response)
    }

    pub(super) async fn start_echoer_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr) {
            return Err(ApiError::generic("Echoer service at this address exists"));
        }

        ctx.start_worker(addr.clone(), Echoer).await?;

        self.registry
            .echoer_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_echoer_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let req_body: StartEchoerServiceRequest = dec.decode()?;

        let addr = req_body.addr.to_string().into();

        let response = match self.start_echoer_service_impl(ctx, addr).await {
            Ok(_) => Response::ok(req.id()),
            Err(_err) => Response::bad_request(req.id()),
        };

        Ok(response)
    }

    pub(super) async fn start_authenticator_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        #[cfg(not(feature = "direct-authenticator"))]
        return Err(ApiError::generic("direct authenticator not available"));

        #[cfg(feature = "direct-authenticator")]
        {
            let body: StartAuthenticatorRequest = dec.decode()?;
            let addr: Address = body.address().into();

            self.start_direct_authenticator_service_impl(ctx, addr, body.path(), body.project())
                .await?;
        }

        Ok(Response::ok(req.id()))
    }

    #[cfg(feature = "direct-authenticator")]
    pub(super) async fn start_direct_authenticator_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        path: &std::path::Path,
        proj: &[u8],
    ) -> Result<()> {
        use crate::nodes::registry::AuthenticatorServiceInfo;
        if self.registry.authenticator_service.contains_key(&addr) {
            return Err(ApiError::generic("authenticator service already started"));
        }
        let db = self.authenticated_storage.async_try_clone().await?;
        let id = self.identity()?.async_try_clone().await?;
        let au = crate::authenticator::direct::Server::new(proj.to_vec(), db, path, id);
        ctx.start_worker(addr.clone(), au).await?;
        self.registry
            .authenticator_service
            .insert(addr, AuthenticatorServiceInfo::default());
        Ok(())
    }
}
