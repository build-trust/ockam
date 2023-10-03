use miette::IntoDiagnostic;
use std::sync::Arc;
use std::time::Duration;

use ockam::compat::sync::Mutex;
use ockam::identity::Identifier;
use ockam::remote::{RemoteRelay, RemoteRelayOptions};
use ockam::Result;
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::{async_trait, Address, AsyncTryClone};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::relay::{CreateRelay, RelayInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::service::in_memory_node::InMemoryNode;
use crate::nodes::BackgroundNode;
use crate::session::sessions::{Replacer, Session};
use crate::session::sessions::{MAX_CONNECT_TIME, MAX_RECOVERY_TIME};

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
            alias,
            at_rust_node,
            authorized,
        } = create_relay;
        match self
            .node_manager
            .create_relay(ctx, &address, alias, at_rust_node, authorized)
            .await
        {
            Ok(body) => Ok(Response::ok(req).body(body)),
            Err(err) => Err(Response::internal_error(
                req,
                &format!("Failed to create relay: {}", err),
            )),
        }
    }

    pub async fn delete_relay(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<RelayInfo>>, Response<Error>> {
        self.node_manager
            .delete_relay(ctx, req, remote_address)
            .await
    }

    pub async fn show_relay(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<RelayInfo>>, Response<Error>> {
        self.node_manager.show_relay(req, remote_address).await
    }

    pub async fn get_relays(
        &self,
        req: &RequestHeader,
    ) -> Result<Response<Vec<RelayInfo>>, Response<Error>> {
        debug!("Handling GetRelays request");
        Ok(Response::ok(req).body(self.node_manager.get_relays().await))
    }
}

impl NodeManager {
    /// This function returns a representation of the relays currently
    /// registered on this node
    pub async fn get_relays(&self) -> Vec<RelayInfo> {
        let relays = self
            .registry
            .relays
            .entries()
            .await
            .iter()
            .map(|(_, registry_info)| RelayInfo::from(registry_info.to_owned()))
            .collect();
        trace!(?relays, "Relays retrieved");
        relays
    }

    /// Create a new Relay
    /// The Connection encapsulates the list of workers required on the relay route.
    /// This route is monitored in the `InMemoryNode` and the workers are restarted if necessary
    /// when the route is unresponsive
    pub async fn create_relay(
        &self,
        ctx: &Context,
        connection: Connection,
        at_rust_node: bool,
        alias: Option<String>,
    ) -> Result<RelayInfo> {
        let route = connection.route(self.tcp_transport()).await?;
        let options = RemoteRelayOptions::new();

        let relay = if at_rust_node {
            if let Some(alias) = alias {
                RemoteRelay::create_static_without_heartbeats(ctx, route, alias, options).await
            } else {
                RemoteRelay::create(ctx, route, options).await
            }
        } else if let Some(alias) = alias {
            RemoteRelay::create_static(ctx, route, alias, options).await
        } else {
            RemoteRelay::create(ctx, route, options).await
        };

        match relay {
            Ok(info) => {
                let registry_info = info.clone();
                let registry_remote_address = registry_info.remote_address().to_string();
                let relay_info = RelayInfo::from(info);
                self.registry
                    .relays
                    .insert(registry_remote_address, registry_info)
                    .await;

                debug!(
                    forwarding_route = %relay_info.forwarding_route(),
                    remote_address = %relay_info.remote_address_ma()?,
                    "CreateRelay request processed, sending back response"
                );
                Ok(relay_info)
            }
            Err(err) => {
                error!(?err, "Failed to create relay");
                Err(err)
            }
        }
    }

    /// This function removes an existing relay based on its remote address
    pub async fn delete_relay(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<RelayInfo>>, Response<Error>> {
        debug!(%remote_address , "Handling DeleteRelay request");

        if let Some(relay_to_delete) = self.registry.relays.remove(remote_address).await {
            debug!(%remote_address, "Successfully removed relay from node registry");

            match ctx
                .stop_worker(relay_to_delete.worker_address().clone())
                .await
            {
                Ok(_) => {
                    debug!(%remote_address, "Successfully stopped relay");
                    Ok(Response::ok(req).body(Some(RelayInfo::from(relay_to_delete.to_owned()))))
                }
                Err(err) => {
                    error!(%remote_address, ?err, "Failed to delete relay from node registry");
                    Err(Response::internal_error(
                        req,
                        &format!("Failed to delete relay at {}: {}", remote_address, err),
                    ))
                }
            }
        } else {
            error!(%remote_address, "Relay not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Relay with address {} not found.", remote_address),
            ))
        }
    }

    /// This function finds an existing relay and returns its configuration
    pub(super) async fn show_relay(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<RelayInfo>>, Response<Error>> {
        debug!("Handling ShowRelay request");
        if let Some(relay) = self.registry.relays.get(remote_address).await {
            debug!(%remote_address, "Relay not found in node registry");
            Ok(Response::ok(req).body(Some(RelayInfo::from(relay.to_owned()))))
        } else {
            error!(%remote_address, "Relay not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Relay with address {} not found.", remote_address),
            ))
        }
    }
}

impl InMemoryNode {
    pub async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        at_rust_node: bool,
        authorized: Option<Identifier>,
    ) -> Result<RelayInfo> {
        debug!(addr = %address, alias = ?alias, at_rust_node = ?at_rust_node, "Handling CreateRelay request");
        let connection_ctx = Arc::new(ctx.async_try_clone().await?);
        let connection = self
            .make_connection(
                connection_ctx.clone(),
                &address.clone(),
                None,
                authorized.clone(),
                None,
                None,
            )
            .await?;
        connection.add_default_consumers(connection_ctx.clone());

        // Add all Hop workers as consumers for Demo purposes
        // Production nodes should not run any Hop workers
        for hop in self.registry.hop_services.keys().await {
            connection.add_consumer(connection_ctx.clone(), &hop);
        }

        let relay = self
            .node_manager
            .create_relay(
                ctx,
                connection.clone(),
                at_rust_node,
                alias.clone().map(|a| a.to_string()),
            )
            .await?;

        if !at_rust_node && !connection.transport_route().is_empty() {
            let ping_route = connection.transport_route().clone();
            let repl = Self::relay_replacer(
                self.node_manager.clone(),
                Arc::new(ctx.async_try_clone().await?),
                connection,
                address.clone(),
                alias,
                authorized,
            );
            let mut session = Session::new(ping_route);
            session.set_replacer(repl);
            self.add_session(session);
        };
        Ok(relay)
    }

    /// Create a session replacer.
    ///
    /// This returns a function that accepts the previous ping address (e.g.
    /// the secure channel worker address) and constructs the whole route
    /// again.
    fn relay_replacer(
        node_manager: Arc<NodeManager>,
        ctx: Arc<Context>,
        connection: Connection,
        addr: MultiAddr,
        alias: Option<String>,
        authorized: Option<Identifier>,
    ) -> Replacer {
        let connection_arc = Arc::new(Mutex::new(connection));
        let node_manager = node_manager.clone();
        Box::new(move |prev_route| {
            let ctx = ctx.clone();
            let addr = addr.clone();
            let alias = alias.clone();
            let authorized = authorized.clone();
            let connection_arc = connection_arc.clone();
            let previous_connection = connection_arc.lock().unwrap().clone();
            let node_manager = node_manager.clone();

            Box::pin(async move {
                debug!(%prev_route, %addr, "creating new remote relay");

                let f = async {
                    for encryptor in &previous_connection.secure_channel_encryptors {
                        if let Err(error) = node_manager
                            .delete_secure_channel(&ctx.clone(), encryptor)
                            .await
                        {
                            //not much we can do about it
                            debug!("cannot delete secure channel `{encryptor}`: {error}");
                        }
                    }
                    if let Some(tcp_connection) = previous_connection.tcp_connection.as_ref() {
                        if let Err(error) = node_manager
                            .tcp_transport
                            .disconnect(tcp_connection.sender_address().clone())
                            .await
                        {
                            debug!("cannot stop tcp worker `{tcp_connection}`: {error}");
                        }
                    }

                    let connection = node_manager
                        .make_connection(
                            ctx.clone(),
                            &addr,
                            None,
                            authorized,
                            None,
                            Some(MAX_CONNECT_TIME),
                        )
                        .await?;
                    connection.add_default_consumers(ctx.clone());
                    *connection_arc.lock().unwrap() = connection.clone();

                    let route = connection.route(node_manager.tcp_transport()).await?;

                    let options = RemoteRelayOptions::new();
                    if let Some(alias) = &alias {
                        RemoteRelay::create_static(&ctx, route, alias, options).await?;
                    } else {
                        RemoteRelay::create(&ctx, route, options).await?;
                    }
                    Ok(connection.transport_route())
                };
                match timeout(MAX_RECOVERY_TIME, f).await {
                    Err(_) => {
                        warn!(%addr, "timeout creating new remote relay");
                        Err(ApiError::core("timeout"))
                    }
                    Ok(Err(e)) => {
                        warn!(%addr, err = %e, "error creating new remote relay");
                        Err(e)
                    }
                    Ok(Ok(a)) => Ok(a),
                }
            })
        })
    }
}

#[async_trait]
pub trait Relays {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        authorized: Option<Identifier>,
    ) -> miette::Result<RelayInfo>;
}

#[async_trait]
impl Relays for BackgroundNode {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        authorized: Option<Identifier>,
    ) -> miette::Result<RelayInfo> {
        let at_rust_node = !address.starts_with(Project::CODE);
        let body = CreateRelay::new(address.clone(), alias, at_rust_node, authorized);
        self.ask(ctx, Request::post("/node/forwarder").body(body))
            .await
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
        credential_name: Option<String>,
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
        credential_name: Option<String>,
        timeout: Option<Duration>,
    ) -> miette::Result<Address> {
        self.node_manager
            .create_secure_channel(
                ctx,
                addr.clone(),
                identity_name,
                Some(vec![authorized]),
                credential_name,
                timeout,
            )
            .await
            .into_diagnostic()
            .map(|sc| sc.encryptor_address().clone())
    }
}

#[async_trait]
impl SecureChannelsCreation for BackgroundNode {
    async fn create_secure_channel(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        authorized: Identifier,
        identity_name: Option<String>,
        credential_name: Option<String>,
        timeout: Option<Duration>,
    ) -> miette::Result<Address> {
        let body = CreateSecureChannelRequest::new(
            addr,
            Some(vec![authorized]),
            identity_name,
            credential_name,
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
