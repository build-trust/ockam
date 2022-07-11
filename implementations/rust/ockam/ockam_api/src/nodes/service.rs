//! Node Manager (Node Man, the superhero that we deserve)

use super::{
    portal::{PortalList, PortalStatus},
    types::{CreateTransport, DeleteTransport},
};
use crate::error::ApiError;
use crate::{nodes::types::*, Method, Request, Response, ResponseBuilder, Status};

use ockam::remote::RemoteForwarder;
use ockam::{Address, Context, Result, Route, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_identity::{Identity, TrustEveryonePolicy};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use crate::auth::Server;
use crate::cloud::enroll::auth0::Auth0TokenProvider;
use crate::identity::IdentityService;
use crate::lmdb::LmdbStorage;
use crate::old::identity::{create_identity, load_identity};
use core::convert::Infallible;
use minicbor::{encode::Write, Decoder};
use ockam_core::route;
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_vault::storage::FileStorage;
use std::path::PathBuf;
use std::sync::Arc;

type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

// TODO: Move to multiaddr implementation
fn invalid_multiaddr_error() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Core, Kind::Invalid, "Invalid multiaddr")
}

// TODO: Move to multiaddr implementation
fn map_multiaddr_err(_err: ockam_multiaddr::Error) -> ockam_core::Error {
    invalid_multiaddr_error()
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan<A>
where
    A: Auth0TokenProvider,
{
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

    pub(crate) auth0_service: A,
}

impl<A> NodeMan<A>
where
    A: Auth0TokenProvider,
{
    /// Create a new NodeMan with the node name from the ockam CLI
    pub async fn create(
        ctx: &Context,
        node_name: String,
        node_dir: PathBuf,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
        auth0_service: A,
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
            auth0_service,
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
}

impl<A> NodeMan<A>
where
    A: Auth0TokenProvider,
{
    pub(crate) fn api_service_route(&self, api_service: &str) -> Route {
        // TODO: add secure channel to the route. It needs changes on the commands side.
        route![api_service]
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

    async fn create_forwarder<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
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
                Response::ok(req.id()).body(b).encode(enc)?;
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .encode(enc)?;
            }
        };
        Ok(())
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

        // TODO: Improve error handling
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

        let response =
            Response::ok(req.id()).body(CreateSecureChannelResponse::new(channel.to_string()));

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

    async fn handle_request<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
        trace! {
            target: "ockam::nodeman::service",
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

        match (method, path_segments.as_slice()) {
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
                .encode(enc)?,

            // ==*== Transports ==*==
            // TODO: Get all transports
            (Get, ["node", "transport"]) => Response::ok(req.id())
                .body(TransportList::new(self.get_transports()))
                .encode(enc)?,
            (Post, ["node", "transport"]) => self.add_transport(req, dec).await?.encode(enc)?,
            (Delete, ["node", "transport"]) => {
                self.delete_transport(req, dec).await?.encode(enc)?
            }

            // ==*== Vault ==*==
            // (Post, ["node", "vault"]) => self.create_vault().await?,

            // ==*== Identity ==*==
            (Post, ["node", "identity"]) => self.create_identity(ctx, req).await?.encode(enc)?,

            // ==*== Secure channels ==*==
            (Post, ["node", "secure_channel"]) => self
                .create_secure_channel(ctx, req, dec)
                .await?
                .encode(enc)?,
            (Post, ["node", "secure_channel_listener"]) => self
                .create_secure_channel_listener(ctx, req, dec)
                .await?
                .encode(enc)?,

            // ==*== Forwarder commands ==*==
            (Post, ["node", "forwarder"]) => self.create_forwarder(ctx, req, dec, enc).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "portal"]) => self.get_portals(req).encode(enc)?,
            (Post, ["node", "portal"]) => self.create_iolet(ctx, req, dec).await?.encode(enc)?,
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Spaces ==*==
            (Post, ["v0", "spaces"]) => self.create_space(ctx, req, dec, enc).await?,
            (Get, ["v0", "spaces"]) => self.list_spaces(ctx, req, enc).await?,
            (Get, ["v0", "spaces", id]) => self.get_space(ctx, req, enc, id).await?,
            (Get, ["v0", "spaces", "name", name]) => {
                self.get_space_by_name(ctx, req, enc, name).await?
            }
            (Delete, ["v0", "spaces", id]) => self.delete_space(ctx, req, enc, id).await?,

            // ==*== Projects ==*==
            (Post, ["v0", "spaces", space_id, "projects"]) => {
                self.create_project(ctx, req, dec, enc, space_id).await?
            }
            (Get, ["v0", "spaces", space_id, "projects"]) => {
                self.list_projects(ctx, req, enc, space_id).await?
            }
            (Get, ["v0", "spaces", space_id, "projects", project_id]) => {
                self.get_project(ctx, req, enc, space_id, project_id)
                    .await?
            }
            (Get, ["v0", "spaces", space_id, "projects", "name", project_name]) => {
                self.get_project_by_name(ctx, req, enc, space_id, project_name)
                    .await?
            }
            (Delete, ["v0", "spaces", space_id, "projects", project_id]) => {
                self.delete_project(ctx, req, enc, space_id, project_id)
                    .await?
            }

            // ==*== Invitations ==*==
            (Post, ["v0", "invitations"]) => self.create_invitation(ctx, req, dec, enc).await?,
            (Get, ["v0", "invitations"]) => self.list_invitations(ctx, req, enc).await?,
            (Put, ["v0", "invitations", id]) => self.accept_invitation(ctx, req, enc, id).await?,
            (Delete, ["v0", "invitations", id]) => {
                self.reject_invitation(ctx, req, enc, id).await?
            }

            // ==*== Enroll ==*==
            (Post, ["v0", "enroll", "auth0"]) => self.enroll_auth0(ctx, req, enc).await?,
            (Get, ["v0", "enroll", "token"]) => {
                self.generate_enrollment_token(ctx, req, dec, enc).await?
            }
            (Put, ["v0", "enroll", "token"]) => {
                self.authenticate_enrollment_token(ctx, req, dec, enc)
                    .await?
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
            }
        }

        Ok(())
    }
}

#[ockam::worker]
impl<A> Worker for NodeMan<A>
where
    A: Auth0TokenProvider,
{
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
        let mut buf = vec![];
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        // If an error occurs, send a response with the error code so the listener can
        // fail fast instead of failing silently here and force the listener to timeout.
        if let Err(err) = self.handle_request(ctx, &req, &mut dec, &mut buf).await {
            error!(?err, "Failed to handle message");
            Response::builder(req.id(), Status::InternalServerError)
                .body(err.to_string())
                .encode(&mut buf)?;
        }
        ctx.send(msg.return_route(), buf).await
    }
}
