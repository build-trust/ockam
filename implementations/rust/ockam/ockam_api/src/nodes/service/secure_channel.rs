use std::str::FromStr;
use std::time::Duration;

use minicbor::Decoder;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::TrustEveryonePolicy;
use ockam::identity::Vault;
use ockam::identity::{
    Identifier, Identities, SecureChannelListenerOptions, SecureChannelOptions, SecureChannels,
    TrustMultiIdentifiersPolicy,
};
use ockam::identity::{SecureChannel, SecureChannelListener};
use ockam::{Address, Result, Route};
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::AsyncTryClone;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cli_state::traits::StateDirTrait;
use crate::cli_state::StateItemTrait;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    DeleteSecureChannelListenerRequest, DeleteSecureChannelListenerResponse,
    DeleteSecureChannelRequest, DeleteSecureChannelResponse, SecureChannelListenersList,
    ShowSecureChannelListenerRequest, ShowSecureChannelListenerResponse, ShowSecureChannelRequest,
    ShowSecureChannelResponse,
};
use crate::nodes::registry::{SecureChannelInfo, SecureChannelListenerInfo};
use crate::nodes::service::NodeIdentities;
use crate::nodes::{NodeManager, NodeManagerWorker};
use crate::DefaultAddress;

/// SECURE CHANNELS
impl NodeManagerWorker {
    pub async fn list_secure_channels(&self, req: &RequestHeader) -> Response<Vec<String>> {
        let secure_channels_info = self.node_manager.list_secure_channels().await;
        Response::ok(req).body(
            secure_channels_info
                .iter()
                .map(|v| v.sc().encryptor_address().to_string())
                .collect(),
        )
    }

    pub(super) async fn create_secure_channel(
        &mut self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response<CreateSecureChannelResponse>, Response<Error>> {
        let CreateSecureChannelRequest {
            addr,
            authorized_identifiers,
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
                    .map(Identifier::try_from)
                    .collect::<Result<Vec<Identifier>>>()?;

                Some(ids)
            }
            None => None,
        };
        let addr = match MultiAddr::from_str(addr.as_str()) {
            Ok(addr) => addr,
            Err(_) => {
                return Err(Response::bad_request(
                    req,
                    &format!("Incorrect multi-address {}", addr),
                ))
            }
        };
        let sc = self
            .node_manager
            .create_secure_channel(
                ctx,
                addr,
                identity,
                authorized_identifiers,
                credential_name,
                timeout,
            )
            .await?;

        let response = Response::ok(req).body(CreateSecureChannelResponse::new(
            sc.encryptor_address(),
            sc.flow_control_id(),
        ));

        Ok(response)
    }

    pub async fn delete_secure_channel(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response<DeleteSecureChannelResponse>, Response<Error>> {
        let body: DeleteSecureChannelRequest = dec.decode()?;
        let addr = Address::from(body.channel);
        info!(%addr, "Handling request to delete secure channel");
        let res = match self.node_manager.delete_secure_channel(ctx, &addr).await {
            Ok(()) => {
                trace!(%addr, "Removed secure channel");
                Some(addr)
            }
            Err(err) => {
                trace!(%addr, %err, "Error removing secure channel");
                None
            }
        };
        Ok(Response::ok(req).body(DeleteSecureChannelResponse::new(res)))
    }

    pub async fn show_secure_channel(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response<ShowSecureChannelResponse>, Response<Error>> {
        let body: ShowSecureChannelRequest = dec.decode()?;
        let sc_address = Address::from(body.channel);
        let info = self.node_manager.get_secure_channel(&sc_address).await;
        Ok(Response::ok(req).body(ShowSecureChannelResponse::new(info)))
    }
}

/// SECURE CHANNEL LISTENERS
impl NodeManagerWorker {
    pub async fn create_secure_channel_listener(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response<()>, Response<Error>> {
        let CreateSecureChannelListenerRequest {
            addr,
            authorized_identifiers,
            vault_name,
            identity_name,
            ..
        } = dec.decode()?;

        let authorized_identifiers = match authorized_identifiers {
            Some(ids) => {
                let ids = ids
                    .into_iter()
                    .map(Identifier::try_from)
                    .collect::<Result<Vec<Identifier>>>()?;

                Some(ids)
            }
            None => None,
        };

        let addr = Address::from(addr);
        if !addr.is_local() {
            return Err(Response::bad_request(
                req,
                &format!("Invalid address: {}", addr),
            ));
        }

        self.node_manager
            .create_secure_channel_listener(
                addr,
                authorized_identifiers,
                vault_name,
                identity_name,
                ctx,
            )
            .await?;

        Ok(Response::ok(req))
    }

    pub async fn delete_secure_channel_listener(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: DeleteSecureChannelListenerRequest = dec.decode()?;
        let addr = Address::from(body.addr);
        Ok(
            match self
                .node_manager
                .delete_secure_channel_listener(ctx, &addr)
                .await
            {
                Some(_) => {
                    trace!(%addr, "Removed secure channel listener");
                    Response::ok(req)
                        .body(DeleteSecureChannelListenerResponse::new(addr))
                        .to_vec()?
                }
                None => {
                    trace!(%addr, "No such secure channel listener to delete");
                    Response::not_found(
                        req,
                        &format!("Secure Channel Listener, {}, not found.", addr),
                    )
                    .to_vec()?
                }
            },
        )
    }

    pub async fn show_secure_channel_listener(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: ShowSecureChannelListenerRequest = dec.decode()?;
        let address = Address::from(body.addr);
        match self
            .node_manager
            .get_secure_channel_listener(&address)
            .await
        {
            Some(info) => Ok(Response::ok(req)
                .body(ShowSecureChannelListenerResponse::new(&info))
                .to_vec()?),
            None => Ok(Response::not_found(
                req,
                &format!("Secure Channel Listener, {}, not found.", address),
            )
            .to_vec()?),
        }
    }

    pub async fn list_secure_channel_listener(
        &self,
        req: &RequestHeader,
    ) -> Response<SecureChannelListenersList> {
        let secure_channel_listeners_info = self.node_manager.list_secure_channel_listeners().await;
        Response::ok(req).body(SecureChannelListenersList::new(
            secure_channel_listeners_info
                .iter()
                .map(ShowSecureChannelListenerResponse::new)
                .collect(),
        ))
    }
}

/// SECURE CHANNELS
impl NodeManager {
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: MultiAddr,
        identity_name: Option<String>,
        authorized_identifiers: Option<Vec<Identifier>>,
        credential_name: Option<String>,
        timeout: Option<Duration>,
    ) -> Result<SecureChannel> {
        let identifier = self.get_identifier(identity_name.clone()).await?;
        let credential = self
            .get_credential(ctx, &identifier, credential_name, timeout)
            .await?;

        let connection_ctx = Arc::new(ctx.async_try_clone().await?);
        let connection = self
            .make_connection(
                connection_ctx,
                &addr,
                Some(identifier.clone()),
                None,
                credential.clone(),
                timeout,
            )
            .await?;
        let sc = self
            .create_secure_channel_internal(
                ctx,
                connection.route(self.tcp_transport()).await?,
                &identifier,
                authorized_identifiers,
                timeout,
                credential,
            )
            .await?;

        // Return secure channel
        Ok(sc)
    }

    pub async fn get_credential(
        &self,
        ctx: &Context,
        identifier: &Identifier,
        credential_name: Option<String>,
        timeout: Option<Duration>,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        debug!("getting a credential");
        let credential = if let Some(credential_name) = credential_name {
            debug!(
                "get the credential using a credential name {}",
                &credential_name
            );
            Some(
                self.cli_state
                    .credentials
                    .get(credential_name)?
                    .config()
                    .credential()?,
            )
        } else {
            match self.trust_context().ok() {
                Some(tc) => {
                    if let Some(t) = timeout {
                        ockam_node::compat::timeout(t, tc.get_credential(ctx, identifier))
                            .await
                            .map_err(|e| {
                                ockam_core::Error::new(Origin::Api, Kind::Timeout, e.to_string())
                            })?
                    } else {
                        tc.get_credential(ctx, identifier).await
                    }
                }
                None => None,
            }
        };
        Ok(credential)
    }

    pub(crate) async fn create_secure_channel_internal(
        &self,
        ctx: &Context,
        sc_route: Route,
        identifier: &Identifier,
        authorized_identifiers: Option<Vec<Identifier>>,
        timeout: Option<Duration>,
        credential: Option<CredentialAndPurposeKey>,
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
        } else if let Some(credential) = self.get_credential(ctx, identifier, None, timeout).await?
        {
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
            .insert(sc_route, sc.clone(), authorized_identifiers)
            .await;

        Ok(sc)
    }

    pub async fn delete_secure_channel(&self, ctx: &Context, addr: &Address) -> Result<()> {
        debug!(%addr, "deleting secure channel");
        self.secure_channels.stop_secure_channel(ctx, addr).await?;
        self.registry.secure_channels.remove_by_addr(addr).await;
        Ok(())
    }

    pub async fn get_secure_channel(&self, addr: &Address) -> Option<SecureChannelInfo> {
        debug!(%addr, "On show secure channel");
        self.registry.secure_channels.get_by_addr(addr).await
    }

    pub async fn list_secure_channels(&self) -> Vec<SecureChannelInfo> {
        let registry = &self.registry.secure_channels;
        registry.list().await
    }
}

/// SECURE CHANNEL LISTENERS
impl NodeManager {
    pub async fn create_secure_channel_listener(
        &self,
        address: Address,
        authorized_identifiers: Option<Vec<Identifier>>,
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

        self.registry
            .secure_channel_listeners
            .insert(
                address.clone(),
                SecureChannelListenerInfo::new(listener.clone()),
            )
            .await;

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

    pub async fn delete_secure_channel_listener(
        &self,
        ctx: &Context,
        addr: &Address,
    ) -> Option<SecureChannelListenerInfo> {
        debug!("deleting secure channel listener: {addr}");
        let _ = ctx.stop_worker(addr.clone()).await;
        self.registry.secure_channel_listeners.remove(addr).await
    }

    pub async fn get_secure_channel_listener(
        &self,
        addr: &Address,
    ) -> Option<SecureChannelListenerInfo> {
        debug!(%addr, "On show secure channel listener");
        self.registry.secure_channel_listeners.get(addr).await
    }

    pub async fn list_secure_channel_listeners(&self) -> Vec<SecureChannelListenerInfo> {
        let registry = &self.registry.secure_channel_listeners;
        registry.values().await
    }
}

impl NodeManager {
    /// Build a SecureChannels struct for a specific vault if one is specified
    /// Otherwise return the shared SecureChannels
    pub(crate) async fn build_secure_channels(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<SecureChannels>> {
        if vault_name.is_none() {
            return Ok(self.secure_channels.clone());
        }
        let vault = self.get_secure_channels_vault(vault_name.clone()).await?;
        let identities = self.get_identities(vault_name).await?;
        let registry = self.secure_channels.secure_channel_registry();
        Ok(SecureChannels::builder()
            .with_vault(vault)
            .with_identities(identities)
            .with_secure_channels_registry(registry)
            .build())
    }

    pub fn node_identities(&self) -> NodeIdentities {
        NodeIdentities::new(self.identities(), self.cli_state.clone())
    }

    pub async fn get_identifier(&self, identity_name: Option<String>) -> Result<Identifier> {
        if let Some(name) = identity_name {
            self.node_identities().get_identifier(name.clone()).await
        } else {
            Ok(self.identifier().clone())
        }
    }

    async fn get_identities(&self, vault_name: Option<String>) -> Result<Arc<Identities>> {
        self.node_identities().get_identities(vault_name).await
    }

    async fn get_secure_channels_vault(&self, vault_name: Option<String>) -> Result<Vault> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(existing_vault)
        } else {
            Ok(self.secure_channels_vault())
        }
    }
}
