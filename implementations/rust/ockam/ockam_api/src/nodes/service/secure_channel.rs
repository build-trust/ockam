use std::time::Duration;

use super::{map_multiaddr_err, NodeManagerWorker};
use crate::error::ApiError;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelListenerRequest, CreateSecureChannelRequest, CreateSecureChannelResponse,
    DeleteSecureChannelListenerRequest, DeleteSecureChannelListenerResponse,
    DeleteSecureChannelRequest, DeleteSecureChannelResponse, PeerNodeType,
    ShowSecureChannelListenerRequest, ShowSecureChannelListenerResponse, ShowSecureChannelRequest,
    ShowSecureChannelResponse,
};
use crate::nodes::registry::Registry;
use crate::nodes::NodeManager;
use crate::{create_tcp_session, DefaultAddress};
use minicbor::Decoder;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Address, Result, Route};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, Sessions};
use ockam_core::{route, CowStr};

use crate::nodes::service::Connection;
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, SecureChannelListenerTrustOptions,
    SecureChannelTrustOptions, TrustMultiIdentifiersPolicy,
};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

impl NodeManager {
    pub(crate) async fn create_secure_channel_internal(
        &mut self,
        identity: &Identity,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        timeout: Option<Duration>,
        session: Option<(Sessions, SessionId)>,
    ) -> Result<Address> {
        // If channel was already created, do nothing.
        if let Some(channel) = self.registry.secure_channels.get_by_route(&sc_route) {
            // Actually should not happen, since every time a new TCP connection is created, so the
            // route is different
            let addr = channel.addr();
            debug!(%addr, "Using cached secure channel");
            return Ok(addr.clone());
        }
        // Else, create it.

        debug!(%sc_route, "Creating secure channel");
        let timeout = timeout.unwrap_or(Duration::from_secs(120));
        let trust_options = SecureChannelTrustOptions::new();

        let trust_options = match session {
            Some((sessions, session_id)) => trust_options.as_consumer(&sessions, &session_id),
            None => trust_options,
        };

        let trust_options = match authorized_identifiers.clone() {
            Some(ids) => trust_options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => trust_options.with_trust_policy(TrustEveryonePolicy),
        };

        let sc_addr = identity
            .create_secure_channel_extended(sc_route.clone(), trust_options, timeout)
            .await?;

        debug!(%sc_route, %sc_addr, "Created secure channel");

        self.registry
            .secure_channels
            .insert(sc_addr.clone(), sc_route, authorized_identifiers);

        Ok(sc_addr)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn create_secure_channel_impl(
        &mut self,
        sc_route: Route,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        timeout: Option<Duration>,
        identity_name: Option<CowStr<'_>>,
        ctx: &Context,
        credential_name: Option<CowStr<'_>>,
        session: Option<(Sessions, SessionId)>,
        peer_node_type: PeerNodeType,
    ) -> Result<Address> {
        let identity: Arc<Identity> = if let Some(identity) = identity_name {
            let idt_state = self.cli_state.identities.get(&identity)?;
            match idt_state.get(ctx, self.vault()?).await {
                Ok(idt) => Arc::new(idt),
                Err(_) => {
                    let default_vault = &self.cli_state.vaults.default()?.get().await?;
                    let vault: Arc<dyn IdentityVault> = Arc::new(default_vault.clone());
                    Arc::new(idt_state.get(ctx, vault).await?)
                }
            }
        } else {
            self.identity.clone()
        };

        let sc_addr = self
            .create_secure_channel_internal(
                &identity,
                sc_route,
                authorized_identifiers,
                timeout,
                session,
            )
            .await?;

        debug!("Deciding if presenting credential");
        // We need to present credential if connecting to anything but an authority, and having
        // a trust_context configured.
        if peer_node_type != PeerNodeType::Authority {
            if let Some(authority) = self
                .trust_context
                .as_ref()
                .and_then(|c| c.authority.as_ref())
            {
                debug!("Had authority");
                if let Some(cred_retriever) = &authority.credential_retriever {
                    debug!(
                        "Had credential retriever, presenting cred for {:?}",
                        identity.identifier()
                    );
                    let credential = if let Some(credential_name) = credential_name {
                        self.cli_state
                            .credentials
                            .get(&credential_name)?
                            .config()
                            .await?
                            .credential()?
                    } else {
                        cred_retriever.credential(&identity).await?
                    };
                    debug!("Credential: {:?}", credential);
                    if peer_node_type == PeerNodeType::Project {
                        debug!(%sc_addr, "One-way credential presentation");
                        identity
                            .present_credential(
                                route![sc_addr.clone(), DefaultAddress::CREDENTIALS_SERVICE],
                                &credential,
                            )
                            .await?;
                        debug!(%sc_addr, "One-way credential presentation success");
                    } else {
                        debug!(%sc_addr, "Mutual credential presentation");
                        identity
                            .present_credential_mutual(
                                route![sc_addr.clone(), DefaultAddress::CREDENTIALS_SERVICE],
                                &authority.identity,
                                self.attributes_storage.clone(),
                                &credential,
                            )
                            .await?;
                        debug!(%sc_addr, "Mutual credential presentation success");
                    }
                }
            }
        }

        // Return secure channel address
        Ok(sc_addr)
    }

    pub(super) async fn create_secure_channel_listener_impl(
        &mut self,
        addr: Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        vault_name: Option<CowStr<'_>>,
        identity_name: Option<CowStr<'_>>,
        ctx: &Context,
    ) -> Result<()> {
        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        let identity: Arc<Identity> = if let Some(identity) = identity_name {
            let idt_state = self.cli_state.identities.get(&identity)?;
            if let Some(vault) = vault_name {
                let default_vault = self.cli_state.vaults.get(&vault)?.get().await?;
                let vault: Arc<dyn IdentityVault> = Arc::new(default_vault.clone());
                Arc::new(idt_state.get(ctx, vault).await?)
            } else {
                Arc::new(idt_state.get(ctx, self.vault()?).await?)
            }
        } else {
            if vault_name.is_some() {
                warn!("The optional vault is ignored when an optional identity is not specified. Using the default identity.");
            }
            self.identity.clone()
        };

        let trust_options = SecureChannelListenerTrustOptions::new(); // FIXME: add session_id
        let trust_options = match authorized_identifiers {
            Some(ids) => trust_options.with_trust_policy(TrustMultiIdentifiersPolicy::new(ids)),
            None => trust_options.with_trust_policy(TrustEveryonePolicy),
        };

        identity
            .create_secure_channel_listener(addr.clone(), trust_options)
            .await?;

        self.registry
            .secure_channel_listeners
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn delete_secure_channel(&mut self, addr: &Address) -> Result<()> {
        debug!(%addr, "deleting secure channel");
        self.identity.stop_secure_channel(addr).await?;
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
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'_>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateSecureChannelRequest {
            addr,
            authorized_identifiers,
            timeout,
            identity_name: identity,
            credential_name,
            peer_node_type,
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

        // Resolve /project multiaddr
        let connection = Connection::new(ctx, &addr);
        let (sc, suffix) = node_manager.connect(connection).await?;

        let channel = if !suffix.is_empty() {
            let full = sc.try_with(&suffix)?;
            debug!("full: {} ", full);
            let tcp_session = create_tcp_session(&full, &node_manager.tcp_transport)
                .await
                .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

            node_manager
                .create_secure_channel_impl(
                    tcp_session.route,
                    authorized_identifiers,
                    timeout,
                    identity,
                    ctx,
                    credential_name,
                    tcp_session.session,
                    peer_node_type,
                )
                .await?
        } else {
            Address::from(sc.first().unwrap().data().to_vec())
        };
        debug!("channel: {:?} ", channel);
        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(&channel));
        Ok(response)
    }

    pub(super) async fn delete_secure_channel(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<DeleteSecureChannelResponse<'_>>> {
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
            .create_secure_channel_listener_impl(addr, authorized_identifiers, vault, identity, ctx)
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
