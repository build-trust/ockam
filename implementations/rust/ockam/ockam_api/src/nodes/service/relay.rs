use std::sync::{Arc, Weak};
use std::time::Duration;

use miette::IntoDiagnostic;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::Identifier;
use ockam::remote::{RemoteRelay, RemoteRelayOptions};
use ockam::Result;
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, Address, AsyncTryClone};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;

use crate::nodes::connection::Connection;
use crate::nodes::models::relay::{CreateRelay, RelayInfo, ReturnTiming};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::registry::RegistryRelayInfo;
use crate::nodes::service::in_memory_node::InMemoryNode;
use crate::nodes::service::secure_channel::SecureChannelType;
use crate::nodes::BackgroundNodeClient;
use crate::session::replacer::{ReplacerOutcome, ReplacerOutputKind, SessionReplacer};
use crate::session::session::Session;

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    pub async fn create_relay(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        create_relay: CreateRelay,
    ) -> Result<Response<RelayInfo>, Response<Error>> {
        let CreateRelay {
            address,
            name,
            authorized,
            relay_address,
            return_timing,
        } = create_relay;

        match self
            .node_manager
            .create_relay(
                ctx,
                &address,
                name.clone(),
                authorized,
                relay_address,
                return_timing,
            )
            .await
        {
            Ok(body) => Ok(Response::ok().with_headers(req).body(body)),
            Err(err) => Err(Response::internal_error(
                req,
                &format!("Failed to create relay at {address} with name {name}. {err}"),
            )),
        }
    }

    /// This function removes an existing relay based on its remote address
    pub async fn delete_relay(
        &self,
        req: &RequestHeader,
        alias: &str,
    ) -> Result<Response<()>, Response<Error>> {
        debug!(%alias , "Handling DeleteRelay request");
        match self.node_manager.delete_relay_impl(alias).await {
            Ok(_) => Ok(Response::ok().with_headers(req).body(())),
            Err(err) => match err.code().kind {
                Kind::NotFound => Err(Response::not_found(
                    req,
                    &format!("Relay with address {alias} not found."),
                )),
                _ => Err(Response::internal_error(
                    req,
                    &format!("Failed to delete relay at {alias}: {err}"),
                )),
            },
        }
    }

    pub async fn show_relay(
        &self,
        req: &RequestHeader,
        alias: &str,
    ) -> Result<Response<RelayInfo>, Response<Error>> {
        self.node_manager.show_relay(req, alias).await
    }

    pub async fn get_relays(
        &self,
        req: &RequestHeader,
    ) -> Result<Response<Vec<RelayInfo>>, Response<Error>> {
        debug!("Handling GetRelays request");
        Ok(Response::ok()
            .with_headers(req)
            .body(self.node_manager.get_relays().await))
    }
}

impl NodeManager {
    /// This function returns a representation of the relays currently
    /// registered on this node
    pub async fn get_relays(&self) -> Vec<RelayInfo> {
        let mut relays = vec![];
        for (_, registry_info) in self.registry.relays.entries().await {
            let session = registry_info.session.lock().await;
            let info = RelayInfo::from_session(
                &session,
                registry_info.destination_address.clone(),
                registry_info.alias.clone(),
            );
            relays.push(info);
        }

        trace!(?relays, "Relays retrieved");
        relays
    }

    /// Create a new Relay
    /// The Connection encapsulates the list of workers required on the relay route.
    /// This route is monitored in the `InMemoryNode` and the workers are restarted if necessary
    /// when the route is unresponsive
    pub async fn create_relay(
        self: &Arc<Self>,
        ctx: &Context,
        address: &MultiAddr,
        alias: String,
        authorized: Option<Identifier>,
        relay_address: Option<String>,
        return_timing: ReturnTiming,
    ) -> Result<RelayInfo> {
        debug!(%alias, %address, ?authorized, ?relay_address, "creating relay");
        if self.registry.relays.contains_key(&alias).await {
            let message = format!("A relay with the name '{alias}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let replacer = RelaySessionReplacer {
            node_manager: Arc::downgrade(self),
            context: ctx.async_try_clone().await?,
            addr: address.clone(),
            relay_address: relay_address.clone(),
            connection: None,
            relay_worker_address: None,
            authorized: authorized.clone(),
        };

        let mut session = Session::create(ctx, Arc::new(Mutex::new(replacer)), None).await?;

        let remote_relay_info = match return_timing {
            ReturnTiming::Immediately => None,
            ReturnTiming::AfterConnection => {
                let result = session
                    .initial_connect()
                    .await
                    .map(|outcome| match outcome {
                        ReplacerOutputKind::Relay(status) => status,
                        _ => {
                            panic!("Unexpected outcome: {:?}", outcome);
                        }
                    });

                match result {
                    Ok(remote_relay_info) => Some(remote_relay_info),
                    Err(err) => {
                        warn!(%err, "Failed to create relay");
                        None
                    }
                }
            }
        };

        session.start_monitoring().await?;

        let relay_info =
            RelayInfo::new(address.clone(), alias.clone(), session.connection_status());
        let relay_info = if let Some(remote_relay_info) = remote_relay_info {
            relay_info.with(remote_relay_info)
        } else {
            relay_info
        };

        let registry_relay_info = RegistryRelayInfo {
            destination_address: address.clone(),
            alias: alias.clone(),
            session: Arc::new(Mutex::new(session)),
        };

        self.registry
            .relays
            .insert(alias.clone(), registry_relay_info.clone())
            .await;

        info!(
            %alias, %address, ?authorized, ?relay_address,
            remote_address = ?relay_info.remote_address(),
            "relay created"
        );

        Ok(relay_info)
    }

    /// Delete a relay.
    ///
    /// This function removes a relay from the node registry and stops the relay worker.
    pub async fn delete_relay_impl(&self, alias: &str) -> Result<(), ockam::Error> {
        if let Some(relay_to_delete) = self.registry.relays.remove(alias).await {
            debug!(%alias, "Successfully removed relay from node registry");
            relay_to_delete.session.lock().await.stop().await;
            debug!(%alias, "Successfully stopped relay");

            Ok(())
        } else {
            error!(%alias, "Relay not found in the node registry");
            Err(ockam::Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("Relay with alias {alias} not found."),
            ))
        }
    }

    /// This function finds an existing relay and returns its configuration
    pub(super) async fn show_relay(
        &self,
        req: &RequestHeader,
        alias: &str,
    ) -> Result<Response<RelayInfo>, Response<Error>> {
        debug!("Handling ShowRelay request");
        if let Some(registry_info) = self.registry.relays.get(alias).await {
            let session = registry_info.session.lock().await;

            let relay_info = RelayInfo::from_session(
                &session,
                registry_info.destination_address.clone(),
                registry_info.alias.clone(),
            );
            Ok(Response::ok().with_headers(req).body(relay_info))
        } else {
            error!(%alias, "Relay not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Relay with alias {alias} not found."),
            ))
        }
    }
}

impl InMemoryNode {
    pub async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: String,
        authorized: Option<Identifier>,
        relay_address: Option<String>,
        return_timing: ReturnTiming,
    ) -> Result<RelayInfo> {
        self.node_manager
            .create_relay(
                ctx,
                address,
                alias,
                authorized,
                relay_address,
                return_timing,
            )
            .await
    }

    pub async fn delete_relay(&self, remote_address: &str) -> Result<()> {
        self.node_manager.delete_relay_impl(remote_address).await
    }
}

struct RelaySessionReplacer {
    node_manager: Weak<NodeManager>,
    context: Context,
    relay_address: Option<String>,

    // current status
    connection: Option<Connection>,
    relay_worker_address: Option<Address>,
    addr: MultiAddr,
    authorized: Option<Identifier>,
}

#[async_trait]
impl SessionReplacer for RelaySessionReplacer {
    async fn create(&mut self) -> Result<ReplacerOutcome> {
        debug!(addr = self.addr.to_string(), relay_address = ?self.relay_address, "Handling CreateRelay request");

        let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
            node_manager
        } else {
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::Cancelled,
                "Node manager is dropped. Can't start a Relay.",
            ));
        };

        let connection = node_manager
            .make_connection(
                &self.context,
                &self.addr.clone(),
                node_manager.identifier(),
                self.authorized.clone(),
                None,
            )
            .await?;
        let connection = self.connection.insert(connection);

        // Add all Hop workers as consumers for Demo purposes
        // Production nodes should not run any Hop workers
        for hop in node_manager.registry.hop_services.keys().await {
            connection.add_consumer(&self.context, &hop);
        }

        let route = connection.route()?;
        let options = RemoteRelayOptions::new();

        let relay_info = if let Some(relay_address) = self.relay_address.as_ref() {
            RemoteRelay::create_static(&self.context, route.clone(), relay_address, options).await
        } else {
            RemoteRelay::create(&self.context, route.clone(), options).await
        }?;

        self.relay_worker_address = Some(relay_info.worker_address().clone());

        // ping directly the other node
        let ping_route = route![connection.transport_route()];

        Ok(ReplacerOutcome {
            ping_route,
            kind: ReplacerOutputKind::Relay(relay_info),
        })
    }

    async fn close(&mut self) {
        let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
            node_manager
        } else {
            warn!("A relay close was issued after the NodeManager shut down, skipping.");
            return;
        };

        if let Some(connection) = self.connection.take() {
            let result = connection.close(&self.context, &node_manager).await;
            if let Err(err) = result {
                error!(?err, "Failed to close connection");
            }
        }

        if let Some(relay_address) = self.relay_worker_address.take() {
            match self.context.stop_worker(relay_address.clone()).await {
                Ok(_) => {
                    debug!(%relay_address, "Successfully stopped relay");
                }
                Err(err) => {
                    error!(%relay_address, ?err, "Failed to stop relay address {relay_address}");
                }
            }
        }
    }
}

#[async_trait]
pub trait Relays {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: String,
        authorized: Option<Identifier>,
        relay_address: Option<String>,
        return_timing: ReturnTiming,
    ) -> miette::Result<RelayInfo>;
}

#[async_trait]
impl Relays for BackgroundNodeClient {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: String,
        authorized: Option<Identifier>,
        relay_address: Option<String>,
        return_timing: ReturnTiming,
    ) -> miette::Result<RelayInfo> {
        let body = CreateRelay::new(
            address.clone(),
            alias,
            authorized,
            relay_address,
            return_timing,
        );
        self.ask(ctx, Request::post("/node/relay").body(body)).await
    }
}

#[async_trait]
pub trait SecureChannelsCreation {
    async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        authorized: Identifier,
        identity_name: Option<String>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> miette::Result<Address>;
}

#[async_trait]
impl SecureChannelsCreation for InMemoryNode {
    async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        authorized: Identifier,
        identity_name: Option<String>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> miette::Result<Address> {
        self.node_manager
            .create_secure_channel(
                ctx,
                addr.clone(),
                identity_name,
                Some(vec![authorized]),
                credential,
                timeout,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await
            .into_diagnostic()
            .map(|sc| sc.encryptor_address().clone())
    }
}

#[async_trait]
impl SecureChannelsCreation for BackgroundNodeClient {
    async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        authorized: Identifier,
        identity_name: Option<String>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> miette::Result<Address> {
        let body = CreateSecureChannelRequest::new(
            addr,
            Some(vec![authorized]),
            identity_name,
            credential,
        );
        let request = Request::post("/node/secure_channel").body(body);
        let response: CreateSecureChannelResponse = if let Some(t) = timeout {
            self.ask_with_timeout(ctx, request, t).await?
        } else {
            self.ask(ctx, request).await?
        };
        Ok(response.addr)
    }
}
