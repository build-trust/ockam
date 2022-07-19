//! Node Manager (Node Man, the superhero that we deserve)

use std::path::PathBuf;
use std::sync::Arc;

use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::{Address, Context, Result, Route, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::route;
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::{Identity, TrustEveryonePolicy};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;

use crate::auth::Server;
use crate::error::ApiError;
use crate::identity::IdentityService;
use crate::lmdb::LmdbStorage;
use crate::old::identity::{create_identity, load_identity};
use crate::{nodes::types::*, Method, Request, Response, ResponseBuilder, Status};

use super::{
    portal::{PortalList, PortalStatus},
    types::{CreateTransport, DeleteTransport},
};

const TARGET: &str = "ockam_api::nodeman::service";

type Alias = String;

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

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan {
    node_name: String,
    node_dir: PathBuf,
    api_transport_id: Alias,
    transports: BTreeMap<Alias, (TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,
    // FIXME: wow this is a terrible way to store data
    portals: BTreeMap<(Alias, PortalType), (String, Option<Route>)>,

    vault: Option<Vault>,
    identity: Option<Identity<Vault>>,
    authenticated_storage: LmdbStorage,
}

impl NodeMan {
    /// Create a new NodeMan with the node name from the ockam CLI
    pub async fn create(
        ctx: &Context,
        node_name: String,
        node_dir: PathBuf,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Result<Self> {
        let api_transport_id = random_alias();
        let portals = BTreeMap::new();
        let mut transports = BTreeMap::new();
        transports.insert(api_transport_id.clone(), api_transport);

        let authenticated_storage =
            LmdbStorage::new(&node_dir.join("authenticated_storage.lmdb")).await?;

        let mut s = Self {
            node_name,
            node_dir,
            api_transport_id,
            transports,
            tcp_transport,
            portals,
            vault: None,
            identity: None,
            authenticated_storage,
        };

        // Each node by default has Vault, with storage inside its directory
        s.create_vault().await?;

        let vault = s
            .vault
            .as_ref()
            .ok_or_else(|| ApiError::generic("Vault doesn't exist"))?;

        // Try to load identity, in case it was already created
        if let Ok(identity) = load_identity(ctx, &s.node_dir, vault).await {
            s.identity = Some(identity);
        }

        Ok(s)
    }

    pub(crate) fn identity(&self) -> Result<&Identity<Vault>> {
        self.identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))
    }
}

impl NodeMan {
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

    //////// Transports API ////////

    // FIXME: return a ResponseBuilder here too!
    fn get_transports(&self) -> Vec<TransportStatus<'_>> {
        self.transports
            .iter()
            .map(|(tid, (tt, tm, addr))| TransportStatus::new(*tt, *tm, addr, tid))
            .collect()
    }

    async fn add_transport<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
        let CreateTransport { tt, tm, addr, .. } = dec.decode()?;

        use {TransportMode::*, TransportType::*};

        info!(
            "Handling request to create a new transport: {}, {}, {}",
            tt, tm, addr
        );
        let addr = addr.to_string();

        let res = match (tt, tm) {
            (Tcp, Listen) => self
                .tcp_transport
                .listen(&addr)
                .await
                .map(|socket| socket.to_string()),
            (Tcp, Connect) => self
                .tcp_transport
                .connect(&addr)
                .await
                .map(|ockam_addr| ockam_addr.to_string()),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok(_) => {
                let tid = random_alias();
                self.transports.insert(tid.clone(), (tt, tm, addr.clone()));
                Response::ok(req.id()).body(TransportStatus::new(tt, tm, addr, tid))
            }
            Err(msg) => Response::bad_request(req.id()).body(TransportStatus::new(
                tt,
                tm,
                msg.to_string(),
                "<none>".to_string(),
            )),
        };

        Ok(response)
    }

    async fn delete_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to delete transport: {}", body.tid);

        let tid: Alias = body.tid.into();

        if self.api_transport_id == tid && !body.force {
            warn!("User requested to delete the API transport without providing force OP flag...");
            return Ok(Response::bad_request(req.id()));
        }

        match self.transports.get(&tid) {
            Some(t) if t.1 == TransportMode::Listen => {
                warn!("It is not currently supported to destroy LISTEN transports");
                Ok(Response::bad_request(req.id()))
            }
            Some(t) => {
                self.tcp_transport.disconnect(&t.2).await?;
                self.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }

    //////// Forwarder API ////////

    async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let CreateForwarder { address, alias, .. } = dec.decode()?;
        let address = Address::from_string(address.to_string());
        debug!(%address, ?alias, "Handling CreateForwarder request");
        let forwarder = match alias {
            Some(alias) => RemoteForwarder::create_static(ctx, address, alias.to_string()).await,
            None => RemoteForwarder::create(ctx, address).await,
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

    //////// Vault API ////////

    async fn create_vault(&mut self) -> Result<()> {
        if self.vault.is_some() {
            return Err(ApiError::generic("Vault already exists"));
        }

        let vault_storage = FileStorage::create(
            &self.node_dir.join("vault.json"),
            &self.node_dir.join("vault.json.temp"),
        )
        .await?;
        let vault = Vault::new(Some(Arc::new(vault_storage)));

        self.vault = Some(vault);

        Ok(())
    }

    //////// Identity API ////////

    async fn create_identity(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
    ) -> Result<ResponseBuilder<CreateIdentityResponse<'_>>> {
        if self.identity.is_some() {
            // TODO: Improve body
            return Ok(Response::bad_request(req.id()).body(CreateIdentityResponse::new("")));
        }

        let vault = self
            .vault
            .as_ref()
            .ok_or_else(|| ApiError::generic("Vault doesn't exist"))?;

        let identity = create_identity(ctx, &self.node_dir, vault, true).await?;
        let identifier = identity.identifier()?.to_string();
        self.identity = Some(identity);

        let response = Response::ok(req.id()).body(CreateIdentityResponse::new(identifier));
        Ok(response)
    }

    //////// Secure channel API ////////

    async fn create_secure_channel<'a>(
        &mut self,
        _ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'a>>> {
        let CreateSecureChannelRequest { addr, .. } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        // TODO: Improve error handling + move logic into CreateSecureChannelRequest
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        let channel = identity
            .create_secure_channel(route, TrustEveryonePolicy, &self.authenticated_storage)
            .await?;

        // TODO: Create Secure Channels Registry

        let response = Response::ok(req.id()).body(CreateSecureChannelResponse::new(channel));

        Ok(response)
    }

    async fn create_secure_channel_listener(
        &mut self,
        _ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let CreateSecureChannelListenerRequest { addr, .. } = dec.decode()?;

        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        // TODO: Should we check if Address is LOCAL?
        let addr = Address::from(addr.as_ref());

        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| ApiError::generic("Identity doesn't exist"))?;

        identity
            .create_secure_channel_listener(addr, TrustEveryonePolicy, &self.authenticated_storage)
            .await?;

        // TODO: Create Secure Channel Listeners Registry

        let response = Response::ok(req.id());

        Ok(response)
    }

    //////// Inlet and Outlet portal API ////////

    fn get_portals(&self, req: &Request<'_>) -> ResponseBuilder<PortalList<'_>> {
        Response::ok(req.id()).body(PortalList::new(
            self.portals
                .iter()
                .map(|((alias, tt), (addr, route))| {
                    PortalStatus::new(
                        *tt,
                        addr,
                        alias,
                        route.as_ref().map(|r| r.to_string().into()),
                    )
                })
                .collect(),
        ))
    }

    async fn create_iolet<'a>(
        &mut self,
        _ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<PortalStatus<'a>>> {
        let CreatePortal {
            addr,
            alias,
            peer: fwd,
            tt,
            ..
        } = dec.decode()?;
        let addr = addr.to_string();
        let alias = alias.map(|a| a.into()).unwrap_or_else(random_alias);

        let res = match tt {
            PortalType::Inlet => {
                info!("Handling request to create inlet portal");
                let fwd = match fwd {
                    Some(f) => f,
                    None => {
                        return Ok(Response::bad_request(req.id())
                            .body(PortalStatus::bad_request(tt, "invalid request payload")))
                    }
                };

                let outlet_route = match Route::parse(fwd) {
                    Some(route) => route,
                    None => {
                        return Ok(Response::bad_request(req.id())
                            .body(PortalStatus::bad_request(tt, "invalid forward route")))
                    }
                };

                self.tcp_transport
                    .create_inlet(addr.clone(), outlet_route)
                    .await
                    .map(|(addr, _)| addr)
            }
            PortalType::Outlet => {
                info!("Handling request to create outlet portal");
                let self_addr = Address::random_local();
                self.tcp_transport
                    .create_outlet(self_addr.clone(), addr.clone())
                    .await
                    .map(|_| self_addr)
            }
        };

        Ok(match res {
            Ok(addr) => {
                Response::ok(req.id()).body(PortalStatus::new(tt, addr.to_string(), alias, None))
            }
            Err(e) => Response::bad_request(req.id()).body(PortalStatus::new(
                tt,
                addr,
                alias,
                Some(e.to_string().into()),
            )),
        })
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
                    "[✓]",
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
            // (Post, ["node", "vault"]) => self.create_vault().await?,

            // ==*== Identity ==*==
            (Post, ["node", "identity"]) => self.create_identity(ctx, req).await?.to_vec()?,

            // ==*== Secure channels ==*==
            (Post, ["node", "secure_channel"]) => {
                self.create_secure_channel(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "secure_channel_listener"]) => self
                .create_secure_channel_listener(ctx, req, dec)
                .await?
                .to_vec()?,

            // ==*== Forwarder commands ==*==
            (Post, ["node", "forwarder"]) => self.create_forwarder(ctx, req, dec).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "portal"]) => self.get_portals(req).to_vec()?,
            (Post, ["node", "portal"]) => self.create_iolet(ctx, req, dec).await?.to_vec()?,
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
            (Post, ["v0", "spaces", space_id, "projects"]) => {
                self.create_project(ctx, req, dec, space_id).await?
            }
            (Get, ["v0", "spaces", space_id, "projects"]) => {
                self.list_projects(ctx, req, dec, space_id).await?
            }
            (Get, ["v0", "spaces", space_id, "projects", project_id]) => {
                self.get_project(ctx, req, dec, space_id, project_id)
                    .await?
            }
            (Get, ["v0", "spaces", space_id, "projects", "name", project_name]) => {
                self.get_project_by_name(ctx, req, dec, space_id, project_name)
                    .await?
            }
            (Delete, ["v0", "spaces", space_id, "projects", project_id]) => {
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
impl Worker for NodeMan {
    type Message = Vec<u8>;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        // By default we start identity and authenticated services

        // TODO: Use existent storage `self.authenticated_storage`
        let s = InMemoryStorage::new();
        let server = Server::new(s);
        ctx.start_worker("authenticated", server).await?;

        // TODO: put that behind some flag or configuration
        // TODO: Use existent vault `self.vault`
        let vault = Vault::create();
        IdentityService::create(ctx, "identity_service", vault).await?;

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
        ctx.send(msg.return_route(), r).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::nodes::NodeMan;
    use ockam::route;

    use super::*;

    impl NodeMan {
        pub(crate) async fn test_create_old(ctx: &Context) -> Result<Route> {
            let node_dir = tempfile::tempdir().unwrap();
            let node_manager = "manager";
            let transport = TcpTransport::create(ctx).await?;
            let node_address = transport.listen("127.0.0.1:0").await?;
            let mut node_man = NodeMan::create(
                ctx,
                "node".to_string(),
                node_dir.into_path(),
                (
                    TransportType::Tcp,
                    TransportMode::Listen,
                    node_address.to_string(),
                ),
                transport,
            )
            .await?;

            // Initialize identity
            let vault = node_man.vault.as_ref().unwrap();
            let identity = create_identity(ctx, &node_man.node_dir, vault, true).await?;
            node_man.identity = Some(identity);

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
            let mut node_man = NodeMan::create(
                ctx,
                "node".to_string(),
                node_dir.into_path(),
                (
                    TransportType::Tcp,
                    TransportMode::Listen,
                    node_address.to_string(),
                ),
                transport,
            )
            .await?;

            // Initialize identity
            let vault = node_man.vault.as_ref().unwrap();
            let identity = create_identity(ctx, &node_man.node_dir, vault, true).await?;
            node_man.identity = Some(identity);

            // Initialize secure channel listener on the mock cloud worker
            node_man
                .identity()?
                .create_secure_channel_listener(
                    "cloud",
                    TrustEveryonePolicy,
                    &node_man.authenticated_storage,
                )
                .await?;
            ctx.start_worker(cloud_address, cloud_worker).await?;

            // Initialize node_man worker and return its route
            ctx.start_worker(node_manager, node_man).await?;
            Ok(route![node_manager])
        }
    }
}
