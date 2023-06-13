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
use ockam_identity::Credential;
use ockam_identity::{SecureChannel, SecureChannelListener};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cli_state::traits::StateDirTrait;
use crate::cli_state::StateItemTrait;
use crate::nodes::connection::Connection;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    CredentialExchangeMode, DeleteSecureChannelListenerRequest,
    DeleteSecureChannelListenerResponse, DeleteSecureChannelRequest, DeleteSecureChannelResponse,
    SecureChannelListenersList, ShowSecureChannelListenerRequest,
    ShowSecureChannelListenerResponse, ShowSecureChannelRequest, ShowSecureChannelResponse,
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
        credential: Option<Credential>,
    ) -> Result<SecureChannel> {
        debug!(%sc_route, "Creating secure channel");
        let options = SecureChannelOptions::new();

        let options = if let Some(timeout) = timeout {
            options.with_timeout(timeout)
        } else {
            options
        };

        let options = if let Some(credential) = credential {
            options.with_credential(credential)
        } else {
            options
        };

        let options = match authorized_identifiers.clone() {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let options = match self.trust_context.clone() {
            Some(trust_context) => options.with_trust_context(trust_context),
            None => options,
        };

        let sc = self
            .secure_channels
            .create_secure_channel(ctx, identifier, sc_route.clone(), options)
            .await?;

        debug!(%sc_route, %sc, "Created secure channel");

        self.registry
            .secure_channels
            .insert(sc_route, sc.clone(), authorized_identifiers);

        Ok(sc)
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
    ) -> Result<SecureChannel> {
        let identifier = self.get_identifier(identity_name.clone()).await?;
        let provided_credential = if let Some(credential_name) = credential_name {
            Some(
                self.cli_state
                    .credentials
                    .get(credential_name)?
                    .config()
                    .credential()?,
            )
        } else {
            None
        };

        // TODO: Determine when we can remove this? Or find a better way to determine
        //       when to check credentials. Currently enable_credential_checks only if a PROJECT AC and PROJECT ID are set
        //       -- Oakley
        let actual_exchange_mode = if self.enable_credential_checks || provided_credential.is_some()
        {
            credential_exchange_mode
        } else {
            CredentialExchangeMode::None
        };

        let credential = match actual_exchange_mode {
            CredentialExchangeMode::None => {
                debug!("No credential presentation");
                None
            }
            CredentialExchangeMode::Oneway | CredentialExchangeMode::Mutual => {
                debug!("One-way credential presentation");
                Some(match provided_credential {
                    Some(c) => c,
                    None => {
                        self.trust_context()?
                            .authority()?
                            .credential(ctx, &identifier)
                            .await?
                    }
                })
            }
        };

        let sc = self
            .create_secure_channel_internal(
                &identifier,
                ctx,
                sc_route,
                authorized_identifiers,
                timeout,
                credential,
            )
            .await?;

        // Return secure channel
        Ok(sc)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        address: Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        vault_name: Option<String>,
        identity_name: Option<String>,
        ctx: &Context,
    ) -> Result<SecureChannelListener> {
        debug!(
            "Handling request to create a new secure channel listener: {}",
            address
        );

        let secure_channels = self.build_secure_channels(vault_name.clone()).await?;
        let identifier = self.get_identifier(identity_name.clone()).await?;

        let options =
            SecureChannelListenerOptions::new().as_consumer(&self.api_transport_flow_control_id);

        let options = match authorized_identifiers {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let options = if let Ok(trust_context) = self.trust_context() {
            options.with_trust_context(trust_context.clone())
        } else {
            options
        };

        let listener = secure_channels
            .create_secure_channel_listener(ctx, &identifier, address.clone(), options)
            .await?;

        self.registry.secure_channel_listeners.insert(
            address.clone(),
            SecureChannelListenerInfo::new(listener.clone()),
        );

        // TODO: Clean
        // Add Echoer, Uppercase and Cred Exch as a consumer by default
        ctx.flow_controls()
            .add_consumer(DefaultAddress::ECHO_SERVICE, listener.flow_control_id());

        ctx.flow_controls().add_consumer(
            DefaultAddress::UPPERCASE_SERVICE,
            listener.flow_control_id(),
        );

        ctx.flow_controls().add_consumer(
            DefaultAddress::CREDENTIALS_SERVICE,
            listener.flow_control_id(),
        );

        Ok(listener)
    }

    /// Build a SecureChannels struct for a specific vault if one is specified
    /// Otherwise return the shared SecureChannels
    pub(crate) async fn build_secure_channels(
        &mut self,
        vault_name: Option<String>,
    ) -> Result<Arc<SecureChannels>> {
        if vault_name.is_none() {
            return Ok(self.secure_channels.clone());
        }
        let vault = self.get_secure_channels_vault(vault_name.clone()).await?;
        let identities = self.get_identities(vault_name).await?;
        let registry = self.secure_channels.secure_channel_registry();
        Ok(SecureChannels::builder()
            .with_identities_vault(vault)
            .with_identities(identities)
            .with_secure_channels_registry(registry)
            .build())
    }

    pub(super) fn node_identities(&self) -> NodeIdentities {
        NodeIdentities::new(self.identities(), self.cli_state.clone())
    }

    pub(crate) async fn get_identifier(
        &self,
        identity_name: Option<String>,
    ) -> Result<IdentityIdentifier> {
        if let Some(name) = identity_name {
            self.node_identities().get_identifier(name.clone()).await
        } else {
            Ok(self.identifier())
        }
    }

    async fn get_identities(&mut self, vault_name: Option<String>) -> Result<Arc<Identities>> {
        self.node_identities().get_identities(vault_name).await
    }

    async fn get_secure_channels_vault(
        &mut self,
        vault_name: Option<String>,
    ) -> Result<Arc<dyn IdentitiesVault>> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(existing_vault)
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
                .map(|v| v.sc().encryptor_address().to_string())
                .collect(),
        )
    }

    pub(super) fn list_secure_channel_listener(
        &self,
        req: &Request<'_>,
        registry: &Registry,
    ) -> ResponseBuilder<SecureChannelListenersList> {
        Response::ok(req.id()).body(SecureChannelListenersList::new(
            registry
                .secure_channel_listeners
                .values()
                .map(ShowSecureChannelListenerResponse::new)
                .collect(),
        ))
    }

    pub(super) async fn create_secure_channel(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse>> {
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

        let sc = node_manager
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
            sc.encryptor_address(),
            sc.flow_control_id(),
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
    ) -> Result<Vec<u8>> {
        let node_manager = self.node_manager.read().await;
        let body: ShowSecureChannelListenerRequest = dec.decode()?;

        let address = Address::from(body.addr.as_ref());

        debug!(%address, "On show secure channel listener");

        match node_manager.registry.secure_channel_listeners.get(&address) {
            Some(info) => Ok(Response::ok(req.id())
                .body(ShowSecureChannelListenerResponse::new(info))
                .to_vec()?),
            None => Ok(Response::not_found(req.id()).to_vec()?),
        }
    }
}
