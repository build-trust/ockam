//! Node Manager (Node Man, the superhero that we deserve)

use std::path::PathBuf;
use std::sync::Arc;

use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::{Address, Context, Result, Route, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::route;
use ockam_identity::{Identity, TrustEveryonePolicy};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;

use super::registry::Registry;
use crate::config::Config;
use crate::error::ApiError;
use crate::lmdb::LmdbStorage;
use crate::nodes::config::NodeManConfig;
use crate::nodes::models::base::NodeStatus;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::transport::{TransportList, TransportMode, TransportType};
use crate::{Method, Request, Response, Status};

mod identity;
mod portals;
mod secure_channel;
mod services;
mod transport;
mod vault;

const TARGET: &str = "ockam_api::nodemanager::service";

pub(crate) type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

// TODO: Move to multiaddr implementation
pub(crate) fn invalid_multiaddr_error() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Core, Kind::Invalid, "Invalid multiaddr")
}

// TODO: Move to multiaddr implementation
pub(crate) fn map_multiaddr_err(_err: ockam_multiaddr::Error) -> ockam_core::Error {
    invalid_multiaddr_error()
}

fn map_anyhow_err(err: anyhow::Error) -> ockam_core::Error {
    ockam_core::Error::new(Origin::Application, Kind::Internal, err)
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeManager {
    node_name: String,
    node_dir: PathBuf,
    config: Config<NodeManConfig>,
    api_transport_id: Alias,
    transports: BTreeMap<Alias, (TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,

    skip_defaults: bool,

    vault: Option<Vault>,
    identity: Option<Identity<Vault>>,
    authenticated_storage: LmdbStorage,

    registry: Registry,
}

impl NodeManager {
    /// Create a new NodeManager with the node name from the ockam CLI
    pub async fn create(
        ctx: &Context,
        node_name: String,
        node_dir: PathBuf,
        skip_defaults: bool,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Result<Self> {
        let api_transport_id = random_alias();
        let mut transports = BTreeMap::new();
        transports.insert(api_transport_id.clone(), api_transport);

        let config = Config::<NodeManConfig>::load(node_dir.clone());

        // Check if we had existing AuthenticatedStorage, create with default location otherwise
        let authenticated_storage_path = config
            .inner()
            .read()
            .unwrap()
            .authenticated_storage_path
            .clone();
        let authenticated_storage = {
            let authenticated_storage_path = match authenticated_storage_path {
                Some(p) => p,
                None => {
                    let default_location = node_dir.join("authenticated_storage.lmdb");

                    config.inner().write().unwrap().authenticated_storage_path =
                        Some(default_location.clone());

                    default_location
                }
            };

            let storage = LmdbStorage::new(&authenticated_storage_path).await?;

            storage
        };

        // Check if we had existing Vault
        let vault_path = config.inner().read().unwrap().vault_path.clone();
        let vault = match vault_path {
            Some(vault_path) => {
                let vault_storage = FileStorage::create(vault_path).await?;
                let vault = Vault::new(Some(Arc::new(vault_storage)));

                Some(vault)
            }
            None => None,
        };

        // Check if we had existing Identity
        let identity_info = config.inner().read().unwrap().identity.clone();
        let identity = match identity_info {
            Some(identity) => match vault.as_ref() {
                Some(vault) => Some(Identity::import(ctx, &identity, vault).await?),
                None => None,
            },
            None => None,
        };

        config.atomic_update().run().map_err(map_anyhow_err)?;

        let mut s = Self {
            node_name,
            node_dir,
            config,
            api_transport_id,
            transports,
            tcp_transport,
            skip_defaults,
            vault,
            identity,
            authenticated_storage,
            registry: Default::default(),
        };

        if !skip_defaults {
            s.create_defaults(ctx).await?;
        }

        Ok(s)
    }

    async fn create_defaults(&mut self, ctx: &Context) -> Result<()> {
        // Create default vault and identity, if they don't exists already
        self.create_vault_impl(None, true).await?;
        self.create_identity_impl(ctx, true).await?;

        Ok(())
    }

    async fn initialize_defaults(&mut self, ctx: &Context) -> Result<()> {
        // Start services
        self.start_vault_service_impl(ctx, "vault_service".into())
            .await?;
        self.start_identity_service_impl(ctx, "identity_service".into())
            .await?;
        self.start_authenticated_service_impl(ctx, "authenticated".into())
            .await?;

        Ok(())
    }
}

impl NodeManager {
    pub(crate) async fn secure_channel(&self, route: impl Into<Route>) -> Result<Address> {
        let route = route.into();
        println!("ddd route {}", route);
        trace!(target: TARGET, %route, "Creating temporary secure channel");
        let channel = self
            .identity()?
            .create_secure_channel(route, TrustEveryonePolicy, &self.authenticated_storage)
            .await?;
        debug!(target: TARGET, %channel, "Temporary secure channel created");
        Ok(channel)
    }

    pub(crate) async fn delete_secure_channel(
        &self,
        ctx: &Context,
        addr: impl Into<Address>,
    ) -> Result<()> {
        ctx.stop_worker(addr).await
    }

    pub(crate) fn cloud_service_route(
        &self,
        address: impl Into<Address>,
        api_service: &str,
    ) -> Route {
        route![address, api_service]
    }

    //////// Forwarder API ////////

    async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let CreateForwarder { address, alias, .. } = dec.decode()?;
        let addr = MultiAddr::try_from(address.0.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;
        debug!(%addr, ?alias, "Handling CreateForwarder request");
        let forwarder = match alias {
            Some(alias) => RemoteForwarder::create_static(ctx, route, alias.to_string()).await,
            None => RemoteForwarder::create(ctx, route).await,
        };
        match forwarder {
            Ok(info) => {
                let b = ForwarderInfo::from(info);
                debug!(
                    forwarding_route = %b.forwarding_route(),
                    remote_address = %b.remote_address(),
                    "CreateForwarder request processed, sending back response"
                );
                Ok(Response::ok(req.id()).body(b).to_vec()?)
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Ok(Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?)
            }
        }
    }

    //////// Request matching and response handling ////////

    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        trace! {
            target: TARGET,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use Method::*;
        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        let r = match (method, path_segments.as_slice()) {
            // ==*== Basic node information ==*==
            // TODO: create, delete, destroy remote nodes
            (Get, ["node"]) => Response::ok(req.id())
                .body(NodeStatus::new(
                    self.node_name.as_str(),
                    "Running",
                    ctx.list_workers().await?.len() as u32,
                    std::process::id() as i32,
                    self.transports.len() as u32,
                ))
                .to_vec()?,

            // ==*== Transports ==*==
            // TODO: Get all transports
            (Get, ["node", "transport"]) => Response::ok(req.id())
                .body(TransportList::new(self.get_transports()))
                .to_vec()?,
            (Post, ["node", "transport"]) => self.add_transport(req, dec).await?.to_vec()?,
            (Delete, ["node", "transport"]) => self.delete_transport(req, dec).await?.to_vec()?,

            // ==*== Vault ==*==
            (Post, ["node", "vault"]) => self.create_vault(req, dec).await?.to_vec()?,

            // ==*== Identity ==*==
            (Post, ["node", "identity"]) => self.create_identity(ctx, req).await?.to_vec()?,
            (Post, ["node", "identity", "actions", "show", "short"]) => {
                self.short_identity(req).await?.to_vec()?
            }
            (Post, ["node", "identity", "actions", "show", "long"]) => {
                self.long_identity(req).await?.to_vec()?
            }

            // ==*== Secure channels ==*==
            (Post, ["node", "secure_channel"]) => {
                self.create_secure_channel(req, dec).await?.to_vec()?
            }
            (Post, ["node", "secure_channel_listener"]) => self
                .create_secure_channel_listener(req, dec)
                .await?
                .to_vec()?,

            // ==*== Services ==*==
            (Post, ["node", "services", "vault"]) => {
                self.start_vault_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "identity"]) => {
                self.start_identity_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "authenticated"]) => self
                .start_authenticated_service(ctx, req, dec)
                .await?
                .to_vec()?,

            // ==*== Forwarder commands ==*==
            (Post, ["node", "forwarder"]) => self.create_forwarder(ctx, req, dec).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => self.get_inlets(req).to_vec()?,
            (Get, ["node", "outlet"]) => self.get_outlets(req).to_vec()?,
            (Post, ["node", "inlet"]) => self.create_inlet(req, dec).await?.to_vec()?,
            (Post, ["node", "outlet"]) => self.create_outlet(req, dec).await?.to_vec()?,
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Spaces ==*==
            (Post, ["v0", "spaces"]) => self.create_space(ctx, req, dec).await?,
            (Get, ["v0", "spaces"]) => self.list_spaces(ctx, req, dec).await?,
            (Get, ["v0", "spaces", id]) => self.get_space(ctx, req, dec, id).await?,
            (Get, ["v0", "spaces", "name", name]) => {
                self.get_space_by_name(ctx, req, dec, name).await?
            }
            (Delete, ["v0", "spaces", id]) => self.delete_space(ctx, req, dec, id).await?,

            // ==*== Projects ==*==
            (Post, ["v0", "projects", space_id]) => {
                self.create_project(ctx, req, dec, space_id).await?
            }
            (Get, ["v0", "projects"]) => self.list_projects(ctx, req, dec).await?,
            (Get, ["v0", "projects", project_id]) => {
                self.get_project(ctx, req, dec, project_id).await?
            }
            // TODO: ockam_command doesn't use this really yet
            (Get, ["v0", "projects", space_id, project_name]) => {
                self.get_project_by_name(ctx, req, dec, space_id, project_name)
                    .await?
            }
            (Delete, ["v0", "projects", space_id, project_id]) => {
                self.delete_project(ctx, req, dec, space_id, project_id)
                    .await?
            }

            // ==*== Invitations ==*==
            (Post, ["v0", "invitations"]) => self.create_invitation(ctx, req, dec).await?,
            (Get, ["v0", "invitations"]) => self.list_invitations(ctx, req, dec).await?,
            (Put, ["v0", "invitations", id]) => self.accept_invitation(ctx, req, dec, id).await?,
            (Delete, ["v0", "invitations", id]) => {
                self.reject_invitation(ctx, req, dec, id).await?
            }

            // ==*== Enroll ==*==
            (Post, ["v0", "enroll", "auth0"]) => self.enroll_auth0(ctx, req, dec).await?,
            (Get, ["v0", "enroll", "token"]) => {
                self.generate_enrollment_token(ctx, req, dec).await?
            }
            (Put, ["v0", "enroll", "token"]) => {
                self.authenticate_enrollment_token(ctx, req, dec).await?
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req.id())
                    .body(format!("Invalid endpoint: {}", path))
                    .to_vec()?
            }
        };
        Ok(r)
    }
}

#[ockam::worker]
impl Worker for NodeManager {
    type Message = Vec<u8>;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        if !self.skip_defaults {
            self.initialize_defaults(ctx).await?;
        }

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let r = match self.handle_request(ctx, &req, &mut dec).await {
            Ok(r) => r,
            // If an error occurs, send a response with the error code so the listener can
            // fail fast instead of failing silently here and force the listener to timeout.
            Err(err) => {
                error!(?err, "Failed to handle message");
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?
            }
        };
        warn!("** sending response");
        ctx.send(msg.return_route(), r).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::nodes::NodeManager;
    use ockam::route;

    use super::*;

    impl NodeManager {
        pub(crate) async fn test_create_old(ctx: &Context) -> Result<Route> {
            let node_dir = tempfile::tempdir().unwrap();
            let node_manager = "manager";
            let transport = TcpTransport::create(ctx).await?;
            let node_address = transport.listen("127.0.0.1:0").await?;
            let mut node_man = NodeManager::create(
                ctx,
                "node".to_string(),
                node_dir.into_path(),
                true,
                (
                    TransportType::Tcp,
                    TransportMode::Listen,
                    node_address.to_string(),
                ),
                transport,
            )
            .await?;

            // Initialize identity
            node_man.create_vault_impl(None, false).await?;
            node_man.create_identity_impl(ctx, false).await?;

            // Initialize node_man worker and return its route
            ctx.start_worker(node_manager, node_man).await?;
            Ok(route![node_manager])
        }

        pub(crate) async fn test_create<W: Worker<Context = Context>>(
            ctx: &Context,
            cloud_address: &str,
            cloud_worker: W,
        ) -> Result<Route> {
            let node_dir = tempfile::tempdir().unwrap();
            let node_manager = "manager";
            let transport = TcpTransport::create(ctx).await?;
            let node_address = transport.listen("127.0.0.1:0").await?;
            let mut node_man = NodeManager::create(
                ctx,
                "node".to_string(),
                node_dir.into_path(),
                true,
                (
                    TransportType::Tcp,
                    TransportMode::Listen,
                    node_address.to_string(),
                ),
                transport,
            )
            .await?;

            node_man.create_vault_impl(None, false).await?;
            node_man.create_identity_impl(ctx, false).await?;

            // Initialize secure channel listener on the mock cloud worker
            node_man
                .create_secure_channel_listener_impl("cloud".into(), None)
                .await?;
            ctx.start_worker(cloud_address, cloud_worker).await?;

            // Initialize node_man worker and return its route
            ctx.start_worker(node_manager, node_man).await?;
            Ok(route![node_manager])
        }
    }
}
