use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::identity::IdentityService;
use crate::nodes::models::services::{
    AuthenticatorType, StartAuthenticatedServiceRequest, StartAuthenticatorRequest,
    StartEchoerServiceRequest, StartIdentityServiceRequest, StartUppercaseServiceRequest,
    StartVaultServiceRequest,
};
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

    pub(super) async fn start_signer_service(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.signer_service.is_some() {
            return Err(ApiError::generic("signing service already started"));
        }
        let ident = if let Some(id) = &self.identity {
            id.clone()
        } else {
            return Err(ApiError::generic("identity not found"));
        };
        let db = self.authenticated_storage.async_try_clone().await?;
        let ss = crate::signer::Server::new(ident, db);
        ctx.start_worker(addr.clone(), ss).await?;
        self.registry.signer_service = Some(addr);
        Ok(())
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

    #[allow(unused_variables)]
    pub(super) async fn start_authenticator_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Result<ResponseBuilder, ResponseBuilder<crate::Error<'a>>>> {
        let body: StartAuthenticatorRequest = dec.decode()?;
        let addr: Address = body.address().into();
        match body.typ() {
            AuthenticatorType::Direct => {
                #[cfg(feature = "direct-authenticator")]
                let res = match self
                    .start_direct_authenticator_service_impl(ctx, addr)
                    .await
                {
                    Ok(()) => Ok(Response::ok(req.id())),
                    Err(e) => {
                        let err = crate::Error::new(req.path()).with_message(e.to_string());
                        Err(Response::bad_request(req.id()).body(err))
                    }
                };
                #[cfg(not(feature = "direct-authenticator"))]
                let res = {
                    let err = crate::Error::new(req.path())
                        .with_message("direct authenticator not available");
                    Err(Response::not_implemented(req.id()).body(err))
                };
                Ok(res)
            }
        }
    }

    #[cfg(feature = "direct-authenticator")]
    pub(super) async fn start_direct_authenticator_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.authenticator_service.contains_key(&addr) {
            return Err(ApiError::generic("authenticator service already started"));
        }
        if let Some(a) = &self.registry.signer_service {
            let ms = self.authenticated_storage.async_try_clone().await?;
            let es = ockam::authenticated_storage::InMemoryStorage::new();
            let sc = crate::signer::Client::new(a.clone().into(), ctx).await?;
            let au = crate::authenticator::direct::Server::new(ms, es, sc);
            ctx.start_worker(addr.clone(), au).await?;
            self.registry.authenticator_service.insert(addr, ());
            Ok(())
        } else {
            Err(ApiError::generic("authenticator depends on signer service"))
        }
    }
}
