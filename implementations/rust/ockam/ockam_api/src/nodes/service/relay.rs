use std::sync::Arc;
use std::time::Duration;

use miette::IntoDiagnostic;

use ockam::identity::Identifier;
use ockam::remote::{RemoteRelay, RemoteRelayOptions};
use ockam::Result;
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, AsyncTryClone};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;

use crate::nodes::connection::Connection;
use crate::nodes::models::relay::{CreateRelay, RelayInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::registry::RegistryRelayInfo;
use crate::nodes::service::in_memory_node::InMemoryNode;
use crate::nodes::BackgroundNodeClient;
use crate::session::sessions::{ReplacerOutcome, ReplacerOutputKind, Session, SessionReplacer};
use crate::session::MedicHandle;

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
            Ok(body) => Ok(Response::ok().with_headers(req).body(body)),
            Err(err) => Err(Response::internal_error(
                req,
                &format!("Failed to create relay: {}", err),
            )),
        }
    }

    /// This function removes an existing relay based on its remote address
    pub async fn delete_relay(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<()>, Response<Error>> {
        debug!(%remote_address , "Handling DeleteRelay request");
        match self.node_manager.delete_relay_impl(remote_address).await {
            Ok(_) => Ok(Response::ok().with_headers(req).body(())),
            Err(err) => match err.code().kind {
                Kind::NotFound => Err(Response::not_found(
                    req,
                    &format!("Relay with address {} not found.", remote_address),
                )),
                _ => Err(Response::internal_error(
                    req,
                    &format!("Failed to delete relay at {}: {}", remote_address, err),
                )),
            },
        }
    }

    pub async fn show_relay(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<RelayInfo>, Response<Error>> {
        self.node_manager.show_relay(req, remote_address).await
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
        let relays = self
            .registry
            .relays
            .entries()
            .await
            .into_iter()
            .map(|(_, registry_info)| registry_info.into())
            .collect();
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
        addr: &MultiAddr,
        alias: Option<String>,
        at_rust_node: bool,
        authorized: Option<Identifier>,
    ) -> Result<RelayInfo> {
        let replacer = RelaySessionReplacer {
            node_manager: self.clone(),
            context: Arc::new(ctx.async_try_clone().await?),
            addr: addr.clone(),
            alias: alias.clone(),
            at_rust_node,
            connection: None,
            relay_address: None,
            authorized,
        };

        let mut session = Session::new(replacer);
        let relay_info =
            MedicHandle::connect(&mut session)
                .await
                .map(|outcome| match outcome.kind {
                    ReplacerOutputKind::Relay(status) => status,
                    _ => {
                        panic!("Unexpected outcome: {:?}", outcome);
                    }
                })?;

        let key = format!("forward_to_{}", alias.as_deref().unwrap_or("default"));
        if self.registry.relays.contains_key(&key).await {
            let message = format!("A relay with the name '{key}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        self.registry
            .relays
            .insert(
                key.clone(),
                RegistryRelayInfo {
                    destination_address: addr.clone(),
                    alias,
                    at_rust_node,
                    session,
                    key,
                },
            )
            .await;

        debug!(
            forwarding_route = %relay_info.forwarding_route(),
            remote_address = %relay_info.remote_address(),
            "CreateRelay request processed, sending back response"
        );

        Ok(relay_info.into())
    }

    /// Delete a relay.
    ///
    /// This function removes a relay from the node registry and stops the relay worker.
    pub async fn delete_relay_impl(&self, remote_address: &str) -> Result<(), ockam::Error> {
        if let Some(relay_to_delete) = self.registry.relays.remove(remote_address).await {
            debug!(%remote_address, "Successfully removed relay from node registry");
            let result = relay_to_delete.session.close().await;
            match result {
                Ok(_) => {
                    debug!(%remote_address, "Successfully stopped relay");
                    Ok(())
                }
                Err(err) => {
                    error!(%remote_address, ?err, "Failed to delete relay from node registry");
                    Err(err)
                }
            }
        } else {
            error!(%remote_address, "Relay not found in the node registry");
            Err(ockam::Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("Relay with address {} not found.", remote_address),
            ))
        }
    }

    /// This function finds an existing relay and returns its configuration
    pub(super) async fn show_relay(
        &self,
        req: &RequestHeader,
        remote_address: &str,
    ) -> Result<Response<RelayInfo>, Response<Error>> {
        debug!("Handling ShowRelay request");
        if let Some(registry_info) = self.registry.relays.get(remote_address).await {
            Ok(Response::ok().with_headers(req).body(registry_info.into()))
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
        self.node_manager
            .create_relay(ctx, address, alias, at_rust_node, authorized)
            .await
    }

    pub async fn delete_relay(&self, remote_address: &str) -> Result<()> {
        self.node_manager.delete_relay_impl(remote_address).await
    }
}

struct RelaySessionReplacer {
    node_manager: Arc<NodeManager>,
    context: Arc<Context>,

    // current status
    connection: Option<Connection>,
    relay_address: Option<Address>,
    alias: Option<String>,
    addr: MultiAddr,
    at_rust_node: bool,
    authorized: Option<Identifier>,
}

#[async_trait]
impl SessionReplacer for RelaySessionReplacer {
    async fn create(&mut self) -> std::result::Result<ReplacerOutcome, ockam_core::Error> {
        debug!(addr = self.addr.to_string(), alias = ?self.alias, at_rust_node = ?self.at_rust_node, "Handling CreateRelay request");
        let connection = self
            .node_manager
            .make_connection(
                self.context.clone(),
                &self.addr.clone(),
                self.node_manager.identifier(),
                self.authorized.clone(),
                None,
                None,
            )
            .await?;
        connection.add_default_consumers(self.context.clone());

        // Add all Hop workers as consumers for Demo purposes
        // Production nodes should not run any Hop workers
        for hop in self.node_manager.registry.hop_services.keys().await {
            connection.add_consumer(self.context.clone(), &hop);
        }

        let route = connection.route()?;
        let options = RemoteRelayOptions::new();

        let relay_info = if self.at_rust_node {
            if let Some(alias) = self.alias.as_ref() {
                RemoteRelay::create_static_without_heartbeats(
                    &self.context,
                    route.clone(),
                    alias,
                    options,
                )
                .await
            } else {
                RemoteRelay::create(&self.context, route.clone(), options).await
            }
        } else if let Some(alias) = self.alias.as_ref() {
            RemoteRelay::create_static(&self.context, route.clone(), alias, options).await
        } else {
            RemoteRelay::create(&self.context, route.clone(), options).await
        }?;

        Ok(ReplacerOutcome {
            ping_route: connection.transport_route(),
            kind: ReplacerOutputKind::Relay(relay_info),
        })
    }

    async fn close(&mut self) -> std::result::Result<(), ockam_core::Error> {
        if let Some(connection) = self.connection.take() {
            connection.close(&self.context, &self.node_manager).await?;
        }

        if let Some(relay_address) = self.relay_address.take() {
            match self.context.stop_worker(relay_address.clone()).await {
                Ok(_) => {
                    debug!(%relay_address, "Successfully stopped relay");
                    Ok(())
                }
                Err(err) => {
                    error!(%relay_address, ?err, "Failed to delete relay from node registry");
                    Err(err)
                }
            }
        } else {
            Ok(())
        }
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
impl Relays for BackgroundNodeClient {
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
impl SecureChannelsCreation for BackgroundNodeClient {
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
