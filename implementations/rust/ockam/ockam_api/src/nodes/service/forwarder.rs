use miette::IntoDiagnostic;
use std::sync::Arc;
use std::time::Duration;

use ockam::compat::sync::Mutex;
use ockam::identity::Identifier;
use ockam::remote::{RemoteForwarder, RemoteForwarderOptions};
use ockam::Result;
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::{async_trait, Address, AsyncTryClone};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::service::in_memory_node::InMemoryNode;
use crate::nodes::BackgroundNode;
use crate::session::sessions::{Replacer, Session};
use crate::session::sessions::{MAX_CONNECT_TIME, MAX_RECOVERY_TIME};

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    pub async fn create_forwarder(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        create_forwarder: CreateForwarder,
    ) -> Result<Response<ForwarderInfo>, Response<Error>> {
        let CreateForwarder {
            address,
            alias,
            at_rust_node,
            authorized,
        } = create_forwarder;
        match self
            .node_manager
            .create_forwarder(ctx, &address, alias, at_rust_node, authorized)
            .await
        {
            Ok(body) => Ok(Response::ok(req).body(body)),
            Err(err) => Err(Response::internal_error(
                req,
                &format!("Failed to create forwarder: {}", err),
            )),
        }
    }

    pub async fn delete_forwarder(
        &self,
        ctx: &mut Context,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<ForwarderInfo>>, Response<Error>> {
        self.node_manager
            .delete_forwarder(ctx, req, remote_address)
            .await
    }

    pub async fn show_forwarder(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<ForwarderInfo>>, Response<Error>> {
        self.node_manager.show_forwarder(req, remote_address).await
    }

    pub async fn get_forwarders(
        &self,
        req: &RequestHeader,
    ) -> Result<Response<Vec<ForwarderInfo>>, Response<Error>> {
        debug!("Handling ListForwarders request");
        Ok(Response::ok(req).body(self.node_manager.get_forwarders().await))
    }
}

impl NodeManager {

    /// This function returns a representation of the relays currently
    /// registered on this node
    pub async fn get_forwarders(&self) -> Vec<ForwarderInfo> {
        let forwarders = self
            .registry
            .forwarders
            .entries()
            .await
            .iter()
            .map(|(_, registry_info)| ForwarderInfo::from(registry_info.to_owned()))
            .collect();
        trace!(?forwarders, "Forwarders retrieved");
        forwarders
    }

    pub async fn create_forwarder(
        &self,
        ctx: &Context,
        connection: Connection,
        at_rust_node: bool,
        alias: Option<String>,
    ) -> Result<ForwarderInfo> {
        let route = connection.route(self.tcp_transport()).await?;
        let options = RemoteForwarderOptions::new();

        let forwarder = if at_rust_node {
            if let Some(alias) = alias {
                RemoteForwarder::create_static_without_heartbeats(ctx, route, alias, options).await
            } else {
                RemoteForwarder::create(ctx, route, options).await
            }
        } else if let Some(alias) = alias {
            RemoteForwarder::create_static(ctx, route, alias, options).await
        } else {
            RemoteForwarder::create(ctx, route, options).await
        };

        match forwarder {
            Ok(info) => {
                let registry_info = info.clone();
                let registry_remote_address = registry_info.remote_address().to_string();
                let forwarder_info = ForwarderInfo::from(info);
                self.registry
                    .forwarders
                    .insert(registry_remote_address, registry_info)
                    .await;

                debug!(
                    forwarding_route = %forwarder_info.forwarding_route(),
                    remote_address = %forwarder_info.remote_address_ma()?,
                    "CreateForwarder request processed, sending back response"
                );
                Ok(forwarder_info)
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Err(err)
            }
        }
    }

    pub(super) async fn delete_forwarder(
        &self,
        ctx: &mut Context,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<ForwarderInfo>>, Response<Error>> {
        debug!(%remote_address , "Handling DeleteForwarder request");

        if let Some(forwarder_to_delete) = self.registry.forwarders.remove(remote_address).await {
            debug!(%remote_address, "Successfully removed forwarder from node registry");

            match ctx
                .stop_worker(forwarder_to_delete.worker_address().clone())
                .await
            {
                Ok(_) => {
                    debug!(%remote_address, "Successfully stopped forwarder");
                    Ok(Response::ok(req)
                        .body(Some(ForwarderInfo::from(forwarder_to_delete.to_owned()))))
                }
                Err(err) => {
                    error!(%remote_address, ?err, "Failed to delete forwarder from node registry");
                    Err(Response::internal_error(
                        req,
                        &format!("Failed to delete forwarder at {}: {}", remote_address, err),
                    ))
                }
            }
        } else {
            error!(%remote_address, "Forwarder not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Forwarder with address {} not found.", remote_address),
            ))
        }
    }

    pub(super) async fn show_forwarder(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<Option<ForwarderInfo>>, Response<Error>> {
        debug!("Handling ShowForwarder request");
        if let Some(forwarder_to_show) = self.registry.forwarders.get(remote_address).await {
            debug!(%remote_address, "Forwarder not found in node registry");
            Ok(Response::ok(req).body(Some(ForwarderInfo::from(forwarder_to_show.to_owned()))))
        } else {
            error!(%remote_address, "Forwarder not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Forwarder with address {} not found.", remote_address),
            ))
        }
    }
}

impl InMemoryNode {
    pub async fn create_forwarder(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        at_rust_node: bool,
        authorized: Option<Identifier>,
    ) -> Result<ForwarderInfo> {
        debug!(addr = %address, alias = ?alias, at_rust_node = ?at_rust_node, "Handling CreateForwarder request");
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

        let forwarder = self
            .node_manager
            .create_forwarder(
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
        Ok(forwarder)
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
                debug!(%prev_route, %addr, "creating new remote forwarder");

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

                    let options = RemoteForwarderOptions::new();
                    if let Some(alias) = &alias {
                        RemoteForwarder::create_static(&ctx, route, alias, options).await?;
                    } else {
                        RemoteForwarder::create(&ctx, route, options).await?;
                    }
                    Ok(connection.transport_route())
                };
                match timeout(MAX_RECOVERY_TIME, f).await {
                    Err(_) => {
                        warn!(%addr, "timeout creating new remote forwarder");
                        Err(ApiError::core("timeout"))
                    }
                    Ok(Err(e)) => {
                        warn!(%addr, err = %e, "error creating new remote forwarder");
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
    ) -> miette::Result<ForwarderInfo>;
}

#[async_trait]
impl Relays for BackgroundNode {
    async fn create_relay(
        &self,
        ctx: &Context,
        address: &MultiAddr,
        alias: Option<String>,
        authorized: Option<Identifier>,
    ) -> miette::Result<ForwarderInfo> {
        let at_rust_node = !address.starts_with(Project::CODE);
        let body = CreateForwarder::new(address.clone(), alias, at_rust_node, authorized);
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
