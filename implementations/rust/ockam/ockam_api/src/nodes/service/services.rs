use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::identity::IdentityService;
use crate::nodes::models::services::{
    ServiceList, ServiceStatus, StartAuthenticatedServiceRequest, StartAuthenticatorRequest,
    StartCredentialsService, StartEchoerServiceRequest, StartIdentityServiceRequest,
    StartOktaIdentityProviderRequest, StartUppercaseServiceRequest, StartVaultServiceRequest,
    StartVerifierService,
};
use crate::nodes::registry::{CredentialsServiceInfo, Registry, VerifierServiceInfo};
use crate::nodes::NodeManager;
use crate::uppercase::Uppercase;
use crate::vault::VaultService;
use minicbor::Decoder;
use ockam::{Address, AsyncTryClone, Context, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn start_vault_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.vault_services.contains_key(&addr) {
            return Err(ApiError::generic("Vault service exists at this address"));
        }

        let vault = self.vault()?.async_try_clone().await?;
        let service = VaultService::new(vault);

        ctx.start_worker(addr.clone(), service).await?;

        self.registry
            .vault_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_identity_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.identity_services.contains_key(&addr) {
            return Err(ApiError::generic("Identity service exists at this address"));
        }

        let vault = self.vault()?.async_try_clone().await?;
        IdentityService::create(ctx, addr.clone(), vault).await?;

        self.registry
            .identity_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_credentials_service_impl<'a>(
        &mut self,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Credentials service exists at this address",
            ));
        }

        let identity = self.identity()?;

        let authorities = self.authorities()?;

        identity
            .start_credentials_exchange_worker(
                authorities.public_identities(),
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

    pub(super) async fn start_authenticated_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.authenticated_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Authenticated service exists at this address",
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

    pub(super) async fn start_uppercase_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Uppercase service exists at this address",
            ));
        }

        ctx.start_worker(addr.clone(), Uppercase).await?;

        self.registry
            .uppercase_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_echoer_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr) {
            return Err(ApiError::generic("Echoer service exists at this address"));
        }

        ctx.start_worker(addr.clone(), Echoer).await?;

        self.registry
            .echoer_services
            .insert(addr, Default::default());

        Ok(())
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
            return Err(ApiError::generic("Authenticator service already started"));
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

    pub(super) async fn start_okta_identity_provider_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        tenant_base_url: &str,
        certificate: &str,
        attributes: &[&str],
        proj: &[u8],
    ) -> Result<()> {
        use crate::nodes::registry::OktaIdentityProviderServiceInfo;
        if self
            .registry
            .okta_identity_provider_services
            .contains_key(&addr)
        {
            return Err(ApiError::generic(
                "Okta Identity Provider service already started",
            ));
        }
        let db = self.authenticated_storage.async_try_clone().await?;
        let au =
            crate::okta::Server::new(proj.to_vec(), db, tenant_base_url, certificate, attributes)?;
        ctx.start_worker(addr.clone(), au).await?;
        self.registry
            .okta_identity_provider_services
            .insert(addr, OktaIdentityProviderServiceInfo::default());
        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) async fn start_vault_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartVaultServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_vault_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_identity_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartIdentityServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_identity_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_authenticated_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartAuthenticatedServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager
            .start_authenticated_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_uppercase_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartUppercaseServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_uppercase_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_echoer_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartEchoerServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_echoer_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_authenticator_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        #[cfg(not(feature = "direct-authenticator"))]
        return Err(ApiError::generic("Direct authenticator not available"));

        #[cfg(feature = "direct-authenticator")]
        {
            let body: StartAuthenticatorRequest = dec.decode()?;
            let addr: Address = body.address().into();

            node_manager
                .start_direct_authenticator_service_impl(ctx, addr, body.path(), body.project())
                .await?;
        }

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_okta_identity_provider_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartOktaIdentityProviderRequest = dec.decode()?;
        let addr: Address = body.address().into();
        node_manager
            .start_okta_identity_provider_service_impl(
                ctx,
                addr,
                body.tenant_base_url(),
                body.certificate(),
                body.attributes(),
                body.project(),
            )
            .await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_verifier_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartVerifierService = dec.decode()?;
        let addr: Address = body.address().into();

        if node_manager.registry.verifier_services.contains_key(&addr) {
            return Err(ApiError::generic("Verifier service exists at this address"));
        }

        let vault = node_manager.vault.async_try_clone().await?;
        let vs = crate::verifier::Verifier::new(vault);
        ctx.start_worker(addr.clone(), vs).await?;

        node_manager
            .registry
            .verifier_services
            .insert(addr, VerifierServiceInfo::default());

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_credentials_service<'a>(
        &mut self,
        _ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();

        node_manager
            .start_credentials_service_impl(addr, oneway)
            .await?;

        Ok(Response::ok(req.id()))
    }

    pub(super) fn list_services<'a>(
        &self,
        req: &Request<'a>,
        registry: &'a Registry,
    ) -> ResponseBuilder<ServiceList<'a>> {
        let mut list = Vec::new();
        registry
            .vault_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "vault")));
        registry
            .identity_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "identity")));
        registry
            .authenticated_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "authenticated")));
        registry
            .uppercase_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "uppercase")));
        registry
            .echoer_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "echoer")));
        registry
            .verifier_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "verifier")));
        registry
            .credentials_services
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "credentials")));

        #[cfg(feature = "direct-authenticator")]
        registry
            .authenticator_service
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "authenticator")));

        Response::ok(req.id()).body(ServiceList::new(list))
    }
}
