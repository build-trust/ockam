use std::time::Duration;

use minicbor::Decoder;

use ockam::identity::TrustEveryonePolicy;
use ockam::identity::{
    Identities, IdentitiesVault, IdentityIdentifier, SecureChannelListenerOptions,
    SecureChannelOptions, SecureChannels, TrustMultiIdentifiersPolicy,
};
use ockam::{Address, Result, Route};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cli_state::traits::StateDirTrait;
use crate::cli_state::{CliStateError, StateItemTrait};
use crate::kafka::KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS;
use crate::nodes::connection::Connection;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    CredentialExchangeMode, DeleteSecureChannelListenerRequest,
    DeleteSecureChannelListenerResponse, DeleteSecureChannelRequest, DeleteSecureChannelResponse,
    ShowSecureChannelListenerRequest, ShowSecureChannelListenerResponse, ShowSecureChannelRequest,
    ShowSecureChannelResponse,
};
use crate::nodes::registry::{Registry, SecureChannelListenerInfo};
use crate::nodes::service::invalid_multiaddr_error;
use crate::nodes::service::NodeIdentities;
use crate::nodes::NodeManager;
use crate::{multiaddr_to_route, DefaultAddress};

use super::{map_multiaddr_err, NodeManagerWorker};

impl NodeManager {
    pub(crate) async fn create_secure_channel_internal(
        &mut self,
        identifier: &IdentityIdentifier,
        ctx: &Context,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        timeout: Option<Duration>,
    ) -> Result<(Address, FlowControlId)> {
        // If channel was already created, do nothing.
        if let Some(channel) = self.registry.secure_channels.get_by_route(&sc_route) {
            // Actually should not happen, since every time a new TCP connection is created, so the
            // route is different
            let addr = channel.addr();
            debug!(%addr, "Using cached secure channel");
            return Ok((addr.clone(), channel.flow_control_id().clone()));
        }
        // Else, create it.

        debug!(%sc_route, "Creating secure channel");
        let timeout = timeout.unwrap_or(Duration::from_secs(120));
        let sc_flow_control_id = FlowControls::generate_id();
        let options = SecureChannelOptions::as_producer(&sc_flow_control_id);

        let options = match authorized_identifiers.clone() {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let sc_addr = self
            .secure_channels
            .create_secure_channel_extended(ctx, identifier, sc_route.clone(), options, timeout)
            .await?;

        debug!(%sc_route, %sc_addr, "Created secure channel");

        self.registry.secure_channels.insert(
            sc_addr.clone(),
            sc_route,
            sc_flow_control_id.clone(),
            authorized_identifiers,
        );

        Ok((sc_addr, sc_flow_control_id))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create_secure_channel_impl(
        &mut self,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        credential_exchange_mode: CredentialExchangeMode,
        timeout: Option<Duration>,
        identity_name: Option<String>,
        ctx: &Context,
        credential_name: Option<String>,
    ) -> Result<(Address, FlowControlId)> {
        let identifier = self.get_identifier(None, identity_name.clone()).await?;
        let provided_credential = if let Some(credential_name) = credential_name {
            Some(
                self.cli_state
                    .credentials
                    .get(&credential_name)?
                    .config()
                    .credential()?,
            )
        } else {
            None
        };

        let (sc_addr, sc_flow_control_id) = self
            .create_secure_channel_internal(
                &identifier,
                ctx,
                sc_route,
                authorized_identifiers,
                timeout,
            )
            .await?;

        // TODO: Determine when we can remove this? Or find a better way to determine
        //       when to check credentials. Currently enable_credential_checks only if a PROJECT AC and PROJECT ID are set
        //       -- Oakley
        let actual_exchange_mode = if self.enable_credential_checks || provided_credential.is_some()
        {
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
                let credential = match provided_credential {
                    Some(c) => c,
                    None => {
                        self.trust_context()?
                            .authority()?
                            .credential(ctx, &identifier)
                            .await?
                    }
                };

                self.credentials_service()
                    .present_credential(
                        ctx,
                        route![sc_addr.clone(), DefaultAddress::CREDENTIALS_SERVICE],
                        credential,
                    )
                    .await?;
                debug!(%sc_addr, "One-way credential presentation success");
            }
            CredentialExchangeMode::Mutual => {
                debug!(%sc_addr, "Mutual credential presentation");
                let credential = match provided_credential {
                    Some(c) => c,
                    None => {
                        self.trust_context()?
                            .authority()?
                            .credential(ctx, &identifier)
                            .await?
                    }
                };

                self.credentials_service()
                    .present_credential_mutual(
                        ctx,
                        route![sc_addr.clone(), DefaultAddress::CREDENTIALS_SERVICE],
                        self.trust_context()?.authorities().await?.as_slice(),
                        credential,
                    )
                    .await?;
                debug!(%sc_addr, "Mutual credential presentation success");
            }
        }

        // Return secure channel address
        Ok((sc_addr, sc_flow_control_id))
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        address: Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        vault_name: Option<String>,
        identity_name: Option<String>,
        ctx: &Context,
    ) -> Result<FlowControlId> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            address
        );

        let secure_channels = self
            .get_secure_channels(vault_name.clone(), identity_name.clone())
            .await?;
        let identifier = self
            .get_identifier(vault_name.clone(), identity_name.clone())
            .await?;

        let flow_control_id = FlowControls::generate_id();
        let options = SecureChannelListenerOptions::new(&flow_control_id);

        let options = match authorized_identifiers {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        secure_channels
            .create_secure_channel_listener(ctx, &identifier, address.clone(), options)
            .await?;

        self.registry.secure_channel_listeners.insert(
            address,
            SecureChannelListenerInfo::new(flow_control_id.clone()),
        );

        // TODO: Clean
        // Add Echoer, Uppercase and Cred Exch as a consumer by default
        ctx.flow_controls().add_consumer(
            DefaultAddress::ECHO_SERVICE,
            &flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        ctx.flow_controls().add_consumer(
            DefaultAddress::UPPERCASE_SERVICE,
            &flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        ctx.flow_controls().add_consumer(
            DefaultAddress::CREDENTIALS_SERVICE,
            &flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        ctx.flow_controls().add_consumer(
            KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS,
            &flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        Ok(flow_control_id)
    }

    pub(crate) async fn get_secure_channels(
        &mut self,
        vault_name: Option<String>,
        identity_name: Option<String>,
    ) -> Result<Arc<SecureChannels>> {
        let secure_channels = if let Some(identity) = identity_name {
            let vault = self.get_secure_channels_vault(vault_name.clone()).await?;
            let identities = self.get_identities(vault_name, identity).await?;
            let registry = self.secure_channels.secure_channel_registry();
            SecureChannels::builder()
                .with_identities_vault(vault)
                .with_identities(identities)
                .with_secure_channels_registry(registry)
                .build()
        } else {
            if vault_name.is_some() {
                warn!("The optional vault is ignored when an optional identity is not specified. Using the default identity.");
            }
            self.secure_channels.clone()
        };
        Ok(secure_channels)
    }

    pub(super) fn node_identities(&self) -> NodeIdentities {
        NodeIdentities::new(self.identities(), self.cli_state.clone())
    }

    pub(crate) async fn get_identifier(
        &self,
        vault_name: Option<String>,
        identity_name: Option<String>,
    ) -> Result<IdentityIdentifier> {
        if let Some(name) = identity_name {
            if let Some(identity) = self
                .node_identities()
                .get_identity(name.clone(), vault_name)
                .await?
            {
                Ok(identity.identifier())
            } else {
                Err(CliStateError::NotFound.into())
            }
        } else {
            Ok(self.identifier())
        }
    }

    async fn get_identities(
        &mut self,
        vault_name: Option<String>,
        identity_name: String,
    ) -> Result<Arc<Identities>> {
        self.node_identities()
            .get_identities(vault_name, identity_name)
            .await
    }

    async fn get_secure_channels_vault(
        &mut self,
        vault_name: Option<String>,
    ) -> Result<Arc<dyn IdentitiesVault>> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(Arc::new(existing_vault))
        } else {
            Ok(self.secure_channels_vault())
        }
    }

    pub(super) async fn delete_secure_channel(
        &mut self,
        ctx: &Context,
        addr: &Address,
    ) -> Result<()> {
        debug!(%addr, "deleting secure channel");
        self.secure_channels.stop_secure_channel(ctx, addr).await?;
        self.registry.secure_channels.remove_by_addr(addr);
        Ok(())
    }

    pub(super) async fn delete_secure_channel_listener_impl(
        &mut self,
        addr: &Address,
    ) -> Result<()> {
        info!("Handling request to delete secure channel listener: {addr}");
        self.registry.secure_channel_listeners.remove(addr);
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

    pub(super) async fn create_secure_channel(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'_, '_>>> {
        let CreateSecureChannelRequest {
            addr,
            authorized_identifiers,
            credential_exchange_mode,
            timeout,
            identity_name: identity,
            credential_name,
            ..
        } = dec.decode()?;

        // credential retrieved from request
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

        let connection = Connection::new(ctx, &addr);
        let connection_instance =
            NodeManager::connect(self.node_manager.clone(), connection).await?;

        let mut node_manager = self.node_manager.write().await;
        let result = multiaddr_to_route(
            &connection_instance.normalized_addr,
            &node_manager.tcp_transport,
        )
        .await
        .ok_or_else(invalid_multiaddr_error)?;

        let (sc_address, sc_flow_control_id) = node_manager
            .create_secure_channel_impl(
                result.route,
                authorized_identifiers,
                credential_exchange_mode,
                timeout,
                identity.map(|i| i.to_string()),
                ctx,
                credential_name.map(|c| c.to_string()),
            )
            .await?;

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(
            &sc_address,
            &sc_flow_control_id,
        ));

        Ok(response)
    }

    pub(super) async fn delete_secure_channel(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<DeleteSecureChannelResponse<'_>>> {
        let body: DeleteSecureChannelRequest = dec.decode()?;
        let addr = Address::from(body.channel.as_ref());
        info!(%addr, "Handling request to delete secure channel");
        let mut node_manager = self.node_manager.write().await;
        let res = match node_manager.delete_secure_channel(ctx, &addr).await {
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

    pub(super) async fn show_secure_channel(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<ShowSecureChannelResponse<'_>>> {
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
        ctx: &Context,
    ) -> Result<ResponseBuilder<()>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateSecureChannelListenerRequest {
            addr,
            authorized_identifiers,
            vault,
            identity,
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

        // TODO: Return to the client side flow_control_id
        node_manager
            .create_secure_channel_listener_impl(
                addr,
                authorized_identifiers,
                vault.map(|v| v.to_string()),
                identity.map(|v| v.to_string()),
                ctx,
            )
            .await?;

        let response = Response::ok(req.id());

        Ok(response)
    }

    pub(super) async fn delete_secure_channel_listener(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<DeleteSecureChannelListenerResponse<'_>>> {
        let body: DeleteSecureChannelListenerRequest = dec.decode()?;
        let addr = Address::from(body.addr.as_ref());
        info!(%addr, "Handling request to delete secure channel listener");
        let mut node_manager = self.node_manager.write().await;
        let res = match node_manager
            .delete_secure_channel_listener_impl(&addr)
            .await
        {
            Ok(()) => {
                trace!(%addr, "Removed secure channel listener");
                Some(addr)
            }
            Err(err) => {
                trace!(%addr, %err, "Error removing secure channel listener");
                None
            }
        };
        Ok(Response::ok(req.id()).body(DeleteSecureChannelListenerResponse::new(res)))
    }

    pub(super) async fn show_secure_channel_listener<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<ShowSecureChannelListenerResponse<'a>>> {
        let node_manager = self.node_manager.read().await;
        let body: ShowSecureChannelListenerRequest = dec.decode()?;

        let address = Address::from(body.addr.as_ref());

        debug!(%address, "On show secure channel listener");

        let _info = node_manager.registry.secure_channel_listeners.get(&address);

        Ok(Response::ok(req.id()).body(ShowSecureChannelListenerResponse::new(&address)))
    }
}
