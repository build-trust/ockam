use std::time::Duration;

use super::{map_multiaddr_err, NodeManagerWorker};
use crate::cli_state::CliState;
use crate::error::ApiError;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    CredentialExchangeMode, DeleteSecureChannelRequest, DeleteSecureChannelResponse,
    ShowSecureChannelRequest, ShowSecureChannelResponse,
};
use crate::nodes::registry::Registry;
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::{route, AsyncTryClone, CowStr};
use ockam_identity::{Identity, IdentityIdentifier, TrustMultiIdentifiersPolicy};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_vault::Vault;

impl NodeManager {
    async fn get_credential_if_needed(&mut self) -> Result<()> {
        let identity = self.identity()?;

        if identity.credential().await.is_some() {
            debug!("Credential check: credential already exists...");
            return Ok(());
        }

        debug!("Credential check: requesting...");
        self.get_credential_impl(false).await?;
        debug!("Credential check: got new credential...");

        Ok(())
    }

    pub(crate) async fn create_secure_channel_internal(
        &mut self,
        identity: &Identity<Vault>,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        timeout: Option<Duration>,
    ) -> Result<Address> {
        // If channel was already created, do nothing.
        if let Some(channel) = self.registry.secure_channels.get_by_route(&sc_route) {
            let addr = channel.addr();
            debug!(%addr, "Using cached secure channel");
            return Ok(addr.clone());
        }
        // Else, create it.

        debug!(%sc_route, "Creating secure channel");
        let timeout = timeout.unwrap_or(Duration::from_secs(120));
        let sc_addr = match authorized_identifiers.clone() {
            Some(ids) => {
                identity
                    .create_secure_channel_extended(
                        sc_route.clone(),
                        TrustMultiIdentifiersPolicy::new(ids),
                        &self.authenticated_storage,
                        timeout,
                        &self.secure_channel_registry,
                    )
                    .await
            }
            None => {
                identity
                    .create_secure_channel_extended(
                        sc_route.clone(),
                        TrustEveryonePolicy,
                        &self.authenticated_storage,
                        timeout,
                        &self.secure_channel_registry,
                    )
                    .await
            }
        }?;

        debug!(%sc_route, %sc_addr, "Created secure channel");

        self.registry
            .secure_channels
            .insert(sc_addr.clone(), sc_route, authorized_identifiers);

        Ok(sc_addr)
    }

    pub(super) async fn create_secure_channel_impl(
        &mut self,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        credential_exchange_mode: CredentialExchangeMode,
        timeout: Option<Duration>,
        identity_name: Option<CowStr<'_>>,
        ctx: &Context,
    ) -> Result<Address> {
        let identity = if let Some(identity) = identity_name {
            let state = CliState::new()?;
            let idt_config = state.identities.get(&identity)?.config;
            match idt_config.get(ctx, self.vault()?).await {
                Ok(idt) => idt,
                Err(_) => {
                    let default_vault = &state.vaults.default()?.config.get().await?;
                    idt_config.get(ctx, default_vault).await?
                }
            }
        } else {
            self.identity()?.async_try_clone().await?
        };

        let sc_addr = self
            .create_secure_channel_internal(&identity, sc_route, authorized_identifiers, timeout)
            .await?;

        let actual_exchange_mode = if self.enable_credential_checks {
            credential_exchange_mode
        } else {
            CredentialExchangeMode::None
        };

        match actual_exchange_mode {
            CredentialExchangeMode::None => {
                debug!(%sc_addr, "No credential presentation");
            }
            CredentialExchangeMode::Oneway => {
                debug!(%sc_addr, "One-way credential presentation");
                self.get_credential_if_needed().await?;
                identity
                    .present_credential(route![sc_addr.clone(), DefaultAddress::CREDENTIAL_SERVICE])
                    .await?;
                debug!(%sc_addr, "One-way credential presentation success");
            }
            CredentialExchangeMode::Mutual => {
                debug!(%sc_addr, "Mutual credential presentation");
                self.get_credential_if_needed().await?;
                let authorities = self.authorities()?;
                identity
                    .present_credential_mutual(
                        route![sc_addr.clone(), DefaultAddress::CREDENTIAL_SERVICE],
                        &authorities.public_identities(),
                        &self.authenticated_storage,
                    )
                    .await?;
                debug!(%sc_addr, "Mutual credential presentation success");
            }
        }

        // Return secure channel address
        Ok(sc_addr)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        addr: Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Result<()> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        let identity = self.identity()?;

        match authorized_identifiers {
            Some(ids) => {
                identity
                    .create_secure_channel_listener(
                        addr.clone(),
                        TrustMultiIdentifiersPolicy::new(ids),
                        &self.authenticated_storage,
                        &self.secure_channel_registry,
                    )
                    .await
            }
            None => {
                identity
                    .create_secure_channel_listener(
                        addr.clone(),
                        TrustEveryonePolicy,
                        &self.authenticated_storage,
                        &self.secure_channel_registry,
                    )
                    .await
            }
        }?;

        self.registry
            .secure_channel_listeners
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn delete_secure_channel(&mut self, addr: &Address) -> Result<()> {
        debug!(%addr, "deleting secure channel");
        let identity = self.identity()?;
        identity.stop_secure_channel(addr).await?;
        self.registry.secure_channels.remove_by_addr(addr);
        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) fn list_secure_channels(
        &self,
        req: &Request<'_>,
        registry: &Registry,
    ) -> ResponseBuilder<Vec<String>> {
        Response::ok(req.id()).body(
            registry
                .secure_channels
                .list()
                .iter()
                .map(|v| v.addr().to_string())
                .collect(),
        )
    }

    pub(super) fn list_secure_channel_listener(
        &self,
        req: &Request<'_>,
        registry: &Registry,
    ) -> ResponseBuilder<Vec<String>> {
        Response::ok(req.id()).body(
            registry
                .secure_channel_listeners
                .keys()
                .map(|addr| addr.to_string())
                .collect(),
        )
    }

    pub(super) async fn create_secure_channel<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateSecureChannelRequest {
            addr,
            authorized_identifiers,
            credential_exchange_mode,
            timeout,
            identity,
            ..
        } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        let authorized_identifiers = match authorized_identifiers {
            Some(ids) => {
                let ids = ids
                    .into_iter()
                    .map(|x| IdentityIdentifier::try_from(x.0.as_ref()))
                    .collect::<Result<Vec<IdentityIdentifier>>>()?;

                Some(ids)
            }
            None => None,
        };

        // TODO: Improve error handling + move logic into CreateSecureChannelRequest
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let channel = node_manager
            .create_secure_channel_impl(
                route,
                authorized_identifiers,
                credential_exchange_mode,
                timeout,
                identity,
                ctx,
            )
            .await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(&channel));

        Ok(response)
    }

    pub(super) async fn delete_secure_channel<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<DeleteSecureChannelResponse<'a>>> {
        let body: DeleteSecureChannelRequest = dec.decode()?;
        let addr = Address::from(body.channel.as_ref());
        info!(%addr, "Handling request to delete secure channel");
        let mut node_manager = self.node_manager.write().await;
        let res = match node_manager.delete_secure_channel(&addr).await {
            Ok(()) => {
                trace!(%addr, "Removed secure channel");
                Some(addr)
            }
            Err(err) => {
                trace!(%addr, %err, "Error removing secure channel");
                None
            }
        };
        Ok(Response::ok(req.id()).body(DeleteSecureChannelResponse::new(res)))
    }

    pub(super) async fn show_secure_channel<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<ShowSecureChannelResponse<'a>>> {
        let node_manager = self.node_manager.read().await;
        let body: ShowSecureChannelRequest = dec.decode()?;

        let sc_address = Address::from(body.channel.as_ref());

        debug!(%sc_address, "On show secure channel");

        let info = node_manager
            .registry
            .secure_channels
            .get_by_addr(&sc_address);

        Ok(Response::ok(req.id()).body(ShowSecureChannelResponse::new(info)))
    }

    pub(super) async fn create_secure_channel_listener(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateSecureChannelListenerRequest {
            addr,
            authorized_identifiers,
            ..
        } = dec.decode()?;

        let authorized_identifiers = match authorized_identifiers {
            Some(ids) => {
                let ids = ids
                    .into_iter()
                    .map(|x| IdentityIdentifier::try_from(x.0.as_ref()))
                    .collect::<Result<Vec<IdentityIdentifier>>>()?;

                Some(ids)
            }
            None => None,
        };

        let addr = Address::from(addr.as_ref());
        if !addr.is_local() {
            return Ok(Response::bad_request(req.id()));
        }

        node_manager
            .create_secure_channel_listener_impl(addr, authorized_identifiers)
            .await?;

        let response = Response::ok(req.id());

        Ok(response)
    }
}
