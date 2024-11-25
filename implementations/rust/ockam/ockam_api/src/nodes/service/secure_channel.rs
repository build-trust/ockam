use std::time::Duration;

use crate::nodes::models::secure_channel::CreateSecureChannelListenerRequest;
use crate::nodes::models::secure_channel::CreateSecureChannelRequest;
use crate::nodes::models::secure_channel::DeleteSecureChannelListenerRequest;
use crate::nodes::models::secure_channel::DeleteSecureChannelRequest;
use crate::nodes::models::secure_channel::ShowSecureChannelListenerRequest;
use crate::nodes::models::secure_channel::ShowSecureChannelRequest;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelResponse, DeleteSecureChannelListenerResponse, DeleteSecureChannelResponse,
    ShowSecureChannelResponse,
};
use crate::nodes::registry::SecureChannelInfo;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::{NodeManager, NodeManagerWorker};
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::Vault;
use ockam::identity::{
    Identifier, Identities, SecureChannelListenerOptions, SecureChannelOptions, SecureChannels,
    TrustMultiIdentifiersPolicy,
};
use ockam::identity::{SecureChannel, SecureChannelListener};
use ockam::identity::{SecureChannelSqlxDatabase, TrustEveryonePolicy};
use ockam::{Address, Result, Route};
use ockam_core::api::{Error, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

#[derive(PartialOrd, PartialEq, Debug)]
pub enum SecureChannelType {
    KeyExchangeAndMessages,
    KeyExchangeOnly,
}

/// SECURE CHANNELS
impl NodeManagerWorker {
    pub async fn list_secure_channels(&self) -> Result<Response<Vec<String>>, Response<Error>> {
        Ok(Response::ok().body(self.node_manager.list_secure_channels().await))
    }

    pub(super) async fn create_secure_channel(
        &mut self,
        create_secure_channel: CreateSecureChannelRequest,
        ctx: &Context,
    ) -> Result<Response<CreateSecureChannelResponse>, Response<Error>> {
        let CreateSecureChannelRequest {
            addr,
            authorized_identifiers,
            timeout,
            identity_name: identity,
            credential,
            ..
        } = create_secure_channel;

        let response = self
            .node_manager
            .create_secure_channel(
                ctx,
                addr,
                identity,
                authorized_identifiers,
                credential,
                timeout,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await
            .map(|secure_channel| {
                Response::ok().body(CreateSecureChannelResponse::new(secure_channel))
            })?;
        Ok(response)
    }

    pub async fn delete_secure_channel(
        &self,
        delete_secure_channel: DeleteSecureChannelRequest,
        ctx: &Context,
    ) -> Result<Response<DeleteSecureChannelResponse>, Response<Error>> {
        let DeleteSecureChannelRequest {
            channel: address, ..
        } = delete_secure_channel;

        let response = self
            .node_manager
            .delete_secure_channel(ctx, &address)
            .await
            .map(|_| Response::ok().body(DeleteSecureChannelResponse::new(Some(address))))?;
        Ok(response)
    }

    pub async fn show_secure_channel(
        &self,
        show_secure_channel: ShowSecureChannelRequest,
    ) -> Result<Response<ShowSecureChannelResponse>, Response<Error>> {
        let ShowSecureChannelRequest { channel: address } = show_secure_channel;

        let response =
            self.node_manager
                .get_secure_channel(&address)
                .await
                .map(|secure_channel| {
                    Response::ok().body(ShowSecureChannelResponse::new(Some(secure_channel)))
                })?;

        Ok(response)
    }
}

/// SECURE CHANNEL LISTENERS
impl NodeManagerWorker {
    pub async fn create_secure_channel_listener(
        &self,
        create_secure_channel_listener: CreateSecureChannelListenerRequest,
        ctx: &Context,
    ) -> Result<Response<()>, Response<Error>> {
        let CreateSecureChannelListenerRequest {
            addr,
            authorized_identifiers,
            identity_name,
            ..
        } = create_secure_channel_listener;

        let response = self
            .node_manager
            .create_secure_channel_listener(
                addr,
                authorized_identifiers,
                identity_name,
                ctx,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await
            .map(|_| Response::ok())?;
        Ok(response)
    }

    pub async fn delete_secure_channel_listener(
        &self,
        delete_secure_channel_listener: DeleteSecureChannelListenerRequest,
        ctx: &Context,
    ) -> Result<Response<DeleteSecureChannelListenerResponse>, Response<Error>> {
        let DeleteSecureChannelListenerRequest { addr } = delete_secure_channel_listener;

        let response = self
            .node_manager
            .delete_secure_channel_listener(ctx, &addr)
            .await
            .map(|_| Response::ok().body(DeleteSecureChannelListenerResponse::new(addr)))?;
        Ok(response)
    }

    pub async fn show_secure_channel_listener(
        &self,
        show_secure_channel_listener: ShowSecureChannelListenerRequest,
    ) -> Result<Response<SecureChannelListener>, Response<Error>> {
        let ShowSecureChannelListenerRequest { addr } = show_secure_channel_listener;
        Ok(Response::ok().body(self.node_manager.get_secure_channel_listener(&addr).await?))
    }

    pub async fn list_secure_channel_listener(
        &self,
    ) -> Result<Response<Vec<SecureChannelListener>>, Response<Error>> {
        Ok(Response::ok().body(self.node_manager.list_secure_channel_listeners().await))
    }
}

/// SECURE CHANNELS
impl NodeManager {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: MultiAddr,
        identity_name: Option<String>,
        authorized_identifiers: Option<Vec<Identifier>>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
        secure_channel_type: SecureChannelType,
    ) -> Result<SecureChannel> {
        let identifier = self.get_identifier_by_name(identity_name.clone()).await?;

        let connection = self
            .make_connection(ctx, &addr, identifier.clone(), None, timeout)
            .await?;
        let sc = self
            .create_secure_channel_internal(
                ctx,
                connection.route()?,
                &identifier,
                authorized_identifiers,
                credential,
                timeout,
                secure_channel_type,
            )
            .await?;

        // Return secure channel
        Ok(sc)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create_secure_channel_internal(
        &self,
        ctx: &Context,
        sc_route: Route,
        identifier: &Identifier,
        authorized_identifiers: Option<Vec<Identifier>>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
        secure_channel_type: SecureChannelType,
    ) -> Result<SecureChannel> {
        debug!(route = %sc_route, %identifier, "initiating secure channel");
        let options = SecureChannelOptions::new();

        let options = if let Some(timeout) = timeout {
            options.with_timeout(timeout)
        } else {
            options
        };

        let options = match self.project_authority() {
            Some(project_authority) => options.with_authority(project_authority),
            None => options,
        };

        let options = if let Some(credential) = credential.as_ref() {
            options.with_credential(credential.clone())?
        } else {
            match self.credential_retriever_creators.project_member.as_ref() {
                None => options,
                Some(credential_retriever_creator) => options
                    .with_credential_retriever_creator(credential_retriever_creator.clone())?,
            }
        };

        let options = match authorized_identifiers.clone() {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let options = if secure_channel_type == SecureChannelType::KeyExchangeOnly {
            // TODO: Should key exchange channels be persisted automatically?
            options.key_exchange_only().persist()?
        } else {
            options
        };

        let sc = self
            .secure_channels
            .create_secure_channel(ctx, identifier, sc_route.clone(), options)
            .await?;

        info!(route = %sc_route, %identifier, "secure channel initiated");

        self.registry
            .secure_channels
            .insert(sc_route, sc.clone(), authorized_identifiers)
            .await;

        Ok(sc)
    }

    pub async fn delete_secure_channel(&self, ctx: &Context, addr: &Address) -> Result<()> {
        debug!(%addr, "deleting secure channel");
        if (self.registry.secure_channels.get_by_addr(addr).await).is_none() {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("Secure channel with address, {}, not found", addr),
            ));
        }
        self.secure_channels.stop_secure_channel(ctx, addr).await?;
        self.registry.secure_channels.remove_by_addr(addr).await;
        Ok(())
    }

    pub async fn get_secure_channel(&self, addr: &Address) -> Result<SecureChannelInfo> {
        debug!(%addr, "On show secure channel");
        self.registry
            .secure_channels
            .get_by_addr(addr)
            .await
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    format!("Secure channel with address, {}, not found", addr),
                )
            })
    }

    pub async fn list_secure_channels(&self) -> Vec<String> {
        let registry = &self.registry.secure_channels;
        let secure_channel_list = registry.list().await;
        secure_channel_list
            .into_iter()
            .map(|secure_channel| secure_channel.sc().encryptor_address().to_string())
            .collect()
    }
}

/// SECURE CHANNEL LISTENERS
impl NodeManager {
    pub(super) async fn start_key_exchanger_service(
        &self,
        context: &Context,
        address: Address,
    ) -> Result<SecureChannelListener> {
        // skip creation if it already exists
        if let Some(listener) = self.registry.secure_channel_listeners.get(&address).await {
            return Ok(listener);
        }

        self.create_secure_channel_listener(
            address.clone(),
            None,
            None,
            context,
            SecureChannelType::KeyExchangeOnly,
        )
        .await
    }

    pub async fn create_secure_channel_listener(
        &self,
        address: Address,
        authorized_identifiers: Option<Vec<Identifier>>,
        identity_name: Option<String>,
        ctx: &Context,
        secure_channel_type: SecureChannelType,
    ) -> Result<SecureChannelListener> {
        debug!(
            "Handling request to create a new secure channel listener: {}",
            address
        );

        let named_identity = match identity_name {
            Some(identity_name) => self.cli_state.get_named_identity(&identity_name).await?,
            None => {
                self.cli_state
                    .get_named_identity_by_identifier(&self.identifier())
                    .await?
            }
        };
        let identifier = named_identity.identifier();
        let named_vault = self
            .cli_state
            .get_named_vault(&named_identity.vault_name())
            .await?;
        let vault = self.cli_state.make_vault(named_vault).await?;
        let secure_channels = self.build_secure_channels(vault).await?;

        let mut options = SecureChannelListenerOptions::new();

        for api_flow_control_id in &self.api_transport_flow_control_ids {
            options = options.as_consumer(api_flow_control_id);
        }

        let options = match authorized_identifiers {
            Some(ids) => options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => options.with_trust_policy(TrustEveryonePolicy),
        };

        let options = match self.project_authority() {
            Some(project_authority) => options.with_authority(project_authority),
            None => options,
        };

        let options = match self.credential_retriever_creators.project_member.as_ref() {
            None => options,
            Some(credential_retriever_creator) => {
                options.with_credential_retriever_creator(credential_retriever_creator.clone())?
            }
        };

        let options = if secure_channel_type == SecureChannelType::KeyExchangeOnly {
            // TODO: Should key exchange channels be persisted automatically?
            options.key_exchange_only().persist()?
        } else {
            options
        };

        let listener = secure_channels
            .create_secure_channel_listener(ctx, &identifier, address.clone(), options)
            .await?;

        info!("Secure channel listener was initialized at {address}");

        self.registry
            .secure_channel_listeners
            .insert(address.clone(), listener.clone())
            .await;

        if secure_channel_type == SecureChannelType::KeyExchangeAndMessages {
            // TODO: Clean
            // Add Echoer as a consumer by default
            ctx.flow_controls()
                .add_consumer(DefaultAddress::ECHO_SERVICE, listener.flow_control_id());

            // TODO: PUNCTURE Make optional?
            ctx.flow_controls().add_consumer(
                DefaultAddress::UDP_PUNCTURE_NEGOTIATION_LISTENER,
                listener.flow_control_id(),
            );

            // Add ourselves to allow tunneling
            ctx.flow_controls()
                .add_consumer(address, listener.flow_control_id());

            ctx.flow_controls().add_consumer(
                DefaultAddress::UPPERCASE_SERVICE,
                listener.flow_control_id(),
            );
        }

        Ok(listener)
    }

    pub async fn delete_secure_channel_listener(
        &self,
        ctx: &Context,
        addr: &Address,
    ) -> Result<SecureChannelListener> {
        debug!("deleting secure channel listener: {addr}");
        ctx.stop_worker(addr.clone()).await?;
        self.registry
            .secure_channel_listeners
            .remove(addr)
            .await
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::Internal,
                    format!("Error while deleting secure channel with addrress {}", addr,),
                )
            })
    }

    pub async fn get_secure_channel_listener(
        &self,
        addr: &Address,
    ) -> Result<SecureChannelListener> {
        debug!(%addr, "On show secure channel listener");
        self.registry
            .secure_channel_listeners
            .get(addr)
            .await
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    format!("Secure channel with address, {}, not found", addr),
                )
            })
    }

    pub async fn list_secure_channel_listeners(&self) -> Vec<SecureChannelListener> {
        let registry = &self.registry.secure_channel_listeners;
        registry.values().await
    }
}

impl NodeManager {
    /// Build a SecureChannels struct for a specific vault
    pub(crate) async fn build_secure_channels(&self, vault: Vault) -> Result<Arc<SecureChannels>> {
        let identities = Identities::create_with_node(self.cli_state.database(), &self.node_name)
            .with_vault(vault)
            .build();
        Ok(Arc::new(SecureChannels::new(
            identities,
            self.secure_channels.secure_channel_registry(),
            SecureChannelSqlxDatabase::make_repository(self.cli_state.database()),
        )))
    }
}
