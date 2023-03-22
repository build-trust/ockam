pub mod types;

use core::{fmt, str};
use lru::LruCache;
use minicbor::{Decode, Decoder, Encode};
use ockam::identity::authenticated_storage::{
    AttributesEntry, IdentityAttributeStorage, IdentityAttributeStorageReader,
};
use ockam::identity::credential::{Credential, OneTimeCode, SchemaId, Timestamp};
use ockam::identity::{Identity, IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_core::api::{self, Error, Method, Request, RequestBuilder, Response, Status};
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, CowStr, DenyAll, Result, Route, Routed, Worker};
use ockam_node::{Context, MessageSendReceiveOptions};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tracing::{trace, warn};
use types::AddMember;

use crate::authenticator::direct::types::CreateToken;

const MAX_TOKEN_DURATION: Duration = Duration::from_secs(600);
const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Schema identifier for a project membership credential.
///
/// The credential will consist of the following attributes:
///
/// - `project_id` : bytes
/// - `role`: b"member"
pub const PROJECT_MEMBER_SCHEMA: SchemaId = SchemaId(1);
pub const PROJECT_ID: &str = "project_id";
pub const LEGACY_MEMBER: &str = "member";

// This acts as a facade, modifying and forwarding incoming messages from legacy clients
// to the new endpoints.   It's going to be removed once we don't need to maintain compatibility
// with old clients anymore.
pub struct LegacyApiConverter {}

impl LegacyApiConverter {
    pub fn new() -> Self {
        Self {}
    }
}

// Keep clippy happy
impl Default for LegacyApiConverter {
    fn default() -> Self {
        LegacyApiConverter::new()
    }
}

#[ockam_core::worker]
impl Worker for LegacyApiConverter {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let body = msg.as_body().clone();
        let mut dec = Decoder::new(&body);
        let mut message = msg.into_local_message();
        let mut second_msg = message.clone(); // Borrow checker.  When authenticating using an enrollment token,
                                              // to adhere to the previous API this legacy worker actually issues
                                              // _two_ request on behalf of the user: one to enroll, other to get
                                              // the credential
        let transport_message = message.transport_mut();

        // Remove my address from the onward_route
        transport_message.onward_route.step()?;

        match dec.decode::<Request>() {
            Ok(req) => match (req.method(), req.path()) {
                (Some(Method::Post), "/tokens") => {
                    transport_message
                        .onward_route
                        .modify()
                        .append("enrollment_token_issuer");
                    ctx.forward(message).await
                }
                (Some(Method::Post), "/members") => {
                    transport_message
                        .onward_route
                        .modify()
                        .append("direct_authenticator");
                    ctx.forward(message).await
                }
                (Some(Method::Post), "/credential") if req.has_body() => {
                    transport_message
                        .onward_route
                        .modify()
                        .append("enrollment_token_acceptor");

                    // We don't want the 200 OK to be routed back to the client here,
                    // as legacy client is expecting the response to contain the credential.
                    transport_message
                        .return_route
                        .modify()
                        .prepend(ctx.address());
                    ctx.forward(message).await?;
                    // Give time for the enrollment to be done before asking for a credential.
                    // A better alternative is to wait for the response on handle_message,
                    // then decode it and issue the next message, then returning the credential
                    // to the client.   But it's too cumbersome for this that is a workaround
                    // to get previous clients to work, and we are removing this code soon after.
                    ockam_node::compat::tokio::time::sleep(Duration::from_millis(2000)).await;
                    // Request the credential,  note the return route points to the client
                    let body = Request::post("/credential").to_vec()?;
                    let transport_message = second_msg.transport_mut();
                    transport_message
                        .onward_route
                        .modify()
                        .append("credential_issuer");
                    transport_message.payload = body;
                    ctx.forward(second_msg).await?;
                    Ok(())
                }
                (Some(Method::Post), "/credential") => {
                    transport_message
                        .onward_route
                        .modify()
                        .append("credential_issuer");
                    ctx.forward(message).await
                }
                (_, _) => {
                    warn!("Legacy Authority Compatibility Worker received request at unknown path: {req:?}");
                    Ok(())
                }
            },
            Err(_) => {
                let mut dec = Decoder::new(&body);
                match dec.decode::<Response>() {
                    Ok(resp) => {
                        if resp.status() == Some(Status::Ok) {
                            debug!("Received resp: {resp:?}");
                        } else {
                            warn!("Received a non-ok response {resp:?}");
                        }
                    }
                    _ => warn!("Received and discarded a non request/response message {message:?}"),
                }
                Ok(())
            }
        }
    }
}

pub struct CredentialIssuer {
    project: Vec<u8>,
    store: Arc<dyn IdentityAttributeStorageReader>,
    ident: Arc<Identity>,
}

impl CredentialIssuer {
    pub async fn new(
        project: Vec<u8>,
        store: Arc<dyn IdentityAttributeStorageReader>,
        identity: Arc<Identity>,
    ) -> Result<Self> {
        Ok(Self {
            project,
            store,
            ident: identity,
        })
    }

    async fn issue_credential(&self, from: &IdentityIdentifier) -> Result<Option<Credential>> {
        match self.store.get_attributes(from).await? {
            Some(entry) => {
                let crd = entry
                    .attrs()
                    .iter()
                    .fold(
                        Credential::builder(from.clone()).with_schema(PROJECT_MEMBER_SCHEMA),
                        |crd, (a, v)| crd.with_attribute(a, v),
                    )
                    .with_attribute(PROJECT_ID, &self.project);
                Ok(Some(self.ident.issue_credential(crd).await?))
            }
            None => Ok(None),
        }
    }
}

#[ockam_core::worker]
impl Worker for CredentialIssuer {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::credential_issuer",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                    match self.issue_credential(from).await {
                        Ok(Some(crd)) => Response::ok(req.id()).body(crd).to_vec()?,
                        Ok(None) => {
                            // Again, this has already been checked by the access control, so if we
                            // reach this point there is an error actually.
                            api::forbidden(&req, "unauthorized member").to_vec()?
                        }
                        Err(error) => api::internal_error(&req, &error.to_string()).to_vec()?,
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

pub struct DirectAuthenticator {
    project: Vec<u8>,
    store: Arc<dyn IdentityAttributeStorage>,
}

impl DirectAuthenticator {
    pub async fn new(project: Vec<u8>, store: Arc<dyn IdentityAttributeStorage>) -> Result<Self> {
        Ok(Self { project, store })
    }

    async fn add_member<'a>(
        &self,
        enroller: &IdentityIdentifier,
        id: &IdentityIdentifier,
        attrs: &HashMap<CowStr<'a>, CowStr<'a>>,
    ) -> Result<()> {
        let auth_attrs = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
            .chain([(PROJECT_ID.to_owned(), self.project.clone())].into_iter())
            .collect();
        let entry = AttributesEntry::new(
            auth_attrs,
            Timestamp::now().unwrap(),
            None,
            Some(enroller.clone()),
        );
        self.store.put_attributes(id, entry).await
    }
}

#[ockam_core::worker]
impl Worker for DirectAuthenticator {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::direct_authenticator",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/members") => {
                    let add: AddMember = dec.decode()?;
                    self.add_member(from, add.member(), add.attributes())
                        .await?;
                    Response::ok(req.id()).to_vec()?
                }
                _ => api::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

#[derive(Clone)]
pub struct EnrollmentTokenAuthenticator {
    project: Vec<u8>,
    tokens: Arc<RwLock<LruCache<[u8; 32], Token>>>,
}

pub struct EnrollmentTokenIssuer(EnrollmentTokenAuthenticator);
pub struct EnrollmentTokenAcceptor(
    EnrollmentTokenAuthenticator,
    Arc<dyn IdentityAttributeStorage>,
);

impl EnrollmentTokenAuthenticator {
    pub fn new_worker_pair(
        project: Vec<u8>,
        storage: Arc<dyn IdentityAttributeStorage>,
    ) -> (EnrollmentTokenIssuer, EnrollmentTokenAcceptor) {
        let base = Self {
            project,
            tokens: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(128).expect("0 < 128"),
            ))),
        };
        (
            EnrollmentTokenIssuer(base.clone()),
            EnrollmentTokenAcceptor(base, storage),
        )
    }
}

impl EnrollmentTokenIssuer {
    async fn issue_token(
        &self,
        enroller: &IdentityIdentifier,
        attrs: HashMap<String, String>,
    ) -> Result<OneTimeCode> {
        let otc = OneTimeCode::new();
        let tkn = Token {
            attrs,
            generated_by: enroller.clone(),
            time: Instant::now(),
        };
        self.0
            .tokens
            .write()
            .map(|mut r| {
                r.put(*otc.code(), tkn);
                otc
            })
            .map_err(|_| {
                ockam_core::Error::new(
                    Origin::Other,
                    Kind::Internal,
                    "failed to get read lock on tokens table",
                )
            })
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenIssuer {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::enrollment_token_issuer",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/tokens") => {
                    let att: CreateToken = dec.decode()?;
                    match self.issue_token(from, att.into_owned_attributes()).await {
                        Ok(otc) => Response::ok(req.id()).body(&otc).to_vec()?,
                        Err(error) => api::internal_error(&req, &error.to_string()).to_vec()?,
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

#[ockam_core::worker]
impl Worker for EnrollmentTokenAcceptor {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        if let Ok(i) = IdentitySecureChannelLocalInfo::find_info(m.local_message()) {
            let from = i.their_identity_id();
            let mut dec = Decoder::new(m.as_body());
            let req: Request = dec.decode()?;
            trace! {
                target: "ockam_api::authenticator::direct::enrollment_token_acceptor",
                from   = %from,
                id     = %req.id(),
                method = ?req.method(),
                path   = %req.path(),
                body   = %req.has_body(),
                "request"
            }
            let res = match (req.method(), req.path()) {
                (Some(Method::Post), "/") | (Some(Method::Post), "/credential") => {
                    //TODO: move out of the worker handle_message implementation
                    let otc: OneTimeCode = dec.decode()?;
                    let token = match self.0.tokens.write() {
                        Ok(mut r) => {
                            if let Some(tkn) = r.pop(otc.code()) {
                                if tkn.time.elapsed() > MAX_TOKEN_DURATION {
                                    Err(api::forbidden(&req, "expired token"))
                                } else {
                                    Ok(tkn)
                                }
                            } else {
                                Err(api::forbidden(&req, "unknown token"))
                            }
                        }
                        Err(_) => Err(api::internal_error(
                            &req,
                            "Failed to get read lock on tokens table",
                        )),
                    };
                    match token {
                        Ok(tkn) => {
                            //TODO: fixme:  unify use of hashmap vs btreemap
                            let attrs = tkn
                                .attrs
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
                                .chain(
                                    [(PROJECT_ID.to_owned(), self.0.project.clone())].into_iter(),
                                )
                                .collect();
                            let entry = AttributesEntry::new(
                                attrs,
                                Timestamp::now().unwrap(),
                                None,
                                Some(tkn.generated_by),
                            );
                            self.1.put_attributes(from, entry).await?;
                            Response::ok(req.id()).to_vec()?
                        }
                        Err(err) => err.to_vec()?,
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
            };
            c.send(m.return_route(), res).await
        } else {
            secure_channel_required(c, m).await
        }
    }
}

struct Token {
    attrs: HashMap<String, String>,
    generated_by: IdentityIdentifier,
    time: Instant,
}

/// Decode, log and map response error to ockam_core error.
fn error(label: &str, res: &Response, dec: &mut Decoder<'_>) -> ockam_core::Error {
    if res.has_body() {
        let err = match dec.decode::<Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target: "ockam_api::authenticator::direct::client",
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            error  = ?err.message(),
            "<- {label}"
        }
        let msg = err.message().unwrap_or(label);
        ockam_core::Error::new(Origin::Application, Kind::Protocol, msg)
    } else {
        ockam_core::Error::new(Origin::Application, Kind::Protocol, label)
    }
}

pub struct CredentialIssuerClient(RpcClient);
impl CredentialIssuerClient {
    pub fn new(client: RpcClient) -> Self {
        CredentialIssuerClient(client)
    }

    pub async fn credential(&self) -> Result<Credential> {
        self.0.request(&Request::post("/")).await
    }
}

pub struct DirectAuthenticatorClient(RpcClient);
impl DirectAuthenticatorClient {
    pub fn new(client: RpcClient) -> Self {
        DirectAuthenticatorClient(client)
    }

    pub async fn add_member(
        &self,
        id: IdentityIdentifier,
        attributes: HashMap<&str, &str>,
    ) -> Result<()> {
        self.0
            .request_no_resp_body(
                &Request::post("/").body(AddMember::new(id).with_attributes(attributes)),
            )
            .await
    }
}

pub struct TokenIssuerClient(RpcClient);
impl TokenIssuerClient {
    pub fn new(client: RpcClient) -> Self {
        TokenIssuerClient(client)
    }

    pub async fn create_token(&self, attributes: HashMap<&str, &str>) -> Result<OneTimeCode> {
        self.0
            .request(&Request::post("/").body(CreateToken::new().with_attributes(attributes)))
            .await
    }
}

pub struct TokenAcceptorClient(RpcClient);
impl TokenAcceptorClient {
    pub fn new(client: RpcClient) -> Self {
        TokenAcceptorClient(client)
    }

    pub async fn present_token(&self, c: &OneTimeCode) -> Result<()> {
        self.0
            .request_no_resp_body(&Request::post("/").body(c))
            .await
    }
}

pub struct RpcClient {
    ctx: Context,
    route: Route,
    timeout: Duration,
}

impl fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcClient")
            .field("route", &self.route)
            .finish()
    }
}

impl RpcClient {
    pub async fn new(r: Route, ctx: &Context) -> Result<Self> {
        let ctx = ctx
            .new_detached(Address::random_tagged("RpcClient"), DenyAll, DenyAll)
            .await?;
        Ok(RpcClient {
            ctx,
            route: r,
            timeout: DEFAULT_CLIENT_TIMEOUT,
        })
    }

    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// Encode request header and body (if any) and send the package to the server.
    async fn request<T, R>(&self, req: &RequestBuilder<'_, T>) -> Result<R>
    where
        T: Encode<()>,
        R: for<'a> Decode<'a, ()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        let vec: Vec<u8> = self
            .ctx
            .send_and_receive_extended(
                self.route.clone(),
                buf,
                MessageSendReceiveOptions::new().with_timeout(self.timeout),
            )
            .await?;
        let mut d = Decoder::new(&vec);
        let resp: Response = d.decode()?;
        if resp.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("new-credential", &resp, &mut d))
        }
    }

    /// Encode request header and body (if any) and send the package to the server.
    async fn request_no_resp_body<T>(&self, req: &RequestBuilder<'_, T>) -> Result<()>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        let vec: Vec<u8> = self.ctx.send_and_receive(self.route.clone(), buf).await?;
        let mut d = Decoder::new(&vec);
        let resp: Response = d.decode()?;
        if resp.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("new-credential", &resp, &mut d))
        }
    }
}

async fn secure_channel_required(c: &mut Context, m: Routed<Vec<u8>>) -> Result<()> {
    // This was, actually, already checked by the access control. So if we reach this point
    // it means there is a bug.  Also, if it' already checked, we should receive the Peer'
    // identity, not an Option to the peer' identity.
    let mut dec = Decoder::new(m.as_body());
    let req: Request = dec.decode()?;
    let res = api::forbidden(&req, "secure channel required").to_vec()?;
    c.send(m.return_route(), res).await
}
