use std::sync::Arc;
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
use ockam_node::Context;

use crate::nodes::connection::Connection;
use crate::nodes::models::relay::{CreateRelay, RelayInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse,
};
use crate::nodes::registry::RegistryRelayInfo;
use crate::nodes::service::in_memory_node::InMemoryNode;
use crate::nodes::service::secure_channel::SecureChannelType;
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
            authorized,
            relay_address,
        } = create_relay;
        match self
            .node_manager
            .create_relay(ctx, &address, alias, authorized, relay_address)
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
        alias: String,
        authorized: Option<Identifier>,
        relay_address: Option<String>,
    ) -> Result<RelayInfo> {
        if self.registry.relays.contains_key(&alias).await {
            let message = format!("A relay with the name '{alias}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let replacer = RelaySessionReplacer {
            node_manager: self.clone(),
            context: Arc::new(ctx.async_try_clone().await?),
            addr: addr.clone(),
            relay_address,
            connection: None,
            relay_worker_address: None,
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

        let registry_relay_info = RegistryRelayInfo {
            destination_address: addr.clone(),
            alias: alias.clone(),
            session,
        };

        self.registry
            .relays
            .insert(alias, registry_relay_info.clone())
            .await;

        debug!(
            forwarding_route = %relay_info.forwarding_route(),
            remote_address = %relay_info.remote_address(),
            "CreateRelay request processed, sending back response"
        );

        Ok(registry_relay_info.into())
    }

    /// Delete a relay.
    ///
    /// This function removes a relay from the node registry and stops the relay worker.
    pub async fn delete_relay_impl(&self, alias: &str) -> Result<(), ockam::Error> {
        if let Some(relay_to_delete) = self.registry.relays.remove(alias).await {
            debug!(%alias, "Successfully removed relay from node registry");
            let result = relay_to_delete.session.close().await;
            match result {
                Ok(_) => {
                    debug!(%alias, "Successfully stopped relay");
                    Ok(())
                }
                Err(err) => {
                    error!(%alias, ?err, "Failed to delete relay from node registry");
                    Err(err)
                }
            }
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
            Ok(Response::ok().with_headers(req).body(registry_info.into()))
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
    ) -> Result<RelayInfo> {
        self.node_manager
            .create_relay(ctx, address, alias, authorized, relay_address)
            .await
    }

    pub async fn delete_relay(&self, remote_address: &str) -> Result<()> {
        self.node_manager.delete_relay_impl(remote_address).await
    }
}

struct RelaySessionReplacer {
    node_manager: Arc<NodeManager>,
    context: Arc<Context>,
    relay_address: Option<String>,

    // current status
    connection: Option<Connection>,
    relay_worker_address: Option<Address>,
    addr: MultiAddr,
    authorized: Option<Identifier>,
}

#[async_trait]
impl SessionReplacer for RelaySessionReplacer {
    async fn create(&mut self) -> std::result::Result<ReplacerOutcome, ockam_core::Error> {
        debug!(addr = self.addr.to_string(), relay_address = ?self.relay_address, "Handling CreateRelay request");
        let connection = self
            .node_manager
            .make_connection(
                self.context.clone(),
                &self.addr.clone(),
                self.node_manager.identifier(),
                self.authorized.clone(),
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
        if let Some(connection) = self.connection.take() {
            let result = connection.close(&self.context, &self.node_manager).await;
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
    ) -> miette::Result<RelayInfo> {
        let body = CreateRelay::new(address.clone(), alias, authorized, relay_address);
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
