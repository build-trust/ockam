pub mod types;

use core::fmt;
use minicbor::encode::{self, Write};
use minicbor::{Decoder, Encode, Encoder};
use ockam_api::{Error, Method, Request, Response, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::io;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Routed, Worker};
use ockam_node::Context;
use tracing::{trace, warn};
use types::{CreateNode, NodeInfo};

#[derive(Debug, Default)]
pub struct Server(HashMap<String, NodeInfo<'static>>);

#[ockam_core::worker]
impl Worker for Server {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let mut buf = Vec::new();
        self.on_request(msg.as_body(), &mut buf)?;
        ctx.send(msg.return_route(), buf).await
    }
}

impl Server {
    pub fn new() -> Self {
        Server::default()
    }

    fn on_request<W>(&mut self, data: &[u8], response: W) -> Result<(), NodesError>
    where
        W: Write<Error = io::Error>,
    {
        let mut dec = Decoder::new(data);
        let req: Request = dec.decode()?;

        match req.method() {
            Some(Method::Get) => match req.path_segments::<2>().as_slice() {
                // Get all nodes:
                [""] => {
                    Encoder::new(response)
                        .encode(Response::new(req.id(), Status::Ok, true))?
                        .encode(encode::ArrayIter::new(self.0.values()))?;
                }
                // Get a single node:
                [id] => {
                    if let Some(n) = self.0.get(*id) {
                        Encoder::new(response)
                            .encode(Response::new(req.id(), Status::Ok, true))?
                            .encode(n)?;
                    } else {
                        Encoder::new(response).encode(Response::new(
                            req.id(),
                            Status::NotFound,
                            false,
                        ))?;
                    }
                }
                _ => {
                    Encoder::new(response)
                        .encode(Response::new(req.id(), Status::BadRequest, true))?
                        .encode(
                            Error::new(req.path())
                                .with_method(Method::Post)
                                .with_message("unknown path"),
                        )?;
                }
            },
            Some(Method::Post) => {
                let cn = dec.decode::<CreateNode>()?;
                // TODO: replace placeholder:
                let ni = NodeInfo::new()
                    .with_name(cn.name().to_string())
                    .with_id("foo".to_string());
                Encoder::new(response)
                    .encode(Response::new(req.id(), Status::Ok, true))?
                    .encode(&ni)?;
                self.0.insert(ni.id().to_string(), ni);
            }
            Some(m) => {
                Encoder::new(response)
                    .encode(Response::new(req.id(), Status::MethodNotAllowed, false))?
                    .encode(Error::new(req.path()).with_method(m))?;
            }
            None => {
                Encoder::new(response)
                    .encode(Response::new(req.id(), Status::NotImplemented, true))?
                    .encode(Error::new(req.path()).with_message("unknown method"))?;
            }
        }

        Ok(())
    }
}

pub struct Client {
    ctx: Context,
    addr: Address,
    buf: Vec<u8>,
}

impl Client {
    pub async fn new(addr: Address, ctx: &Context) -> ockam_core::Result<Self> {
        let ctx = ctx.new_context(Address::random_local()).await?;
        Ok(Client {
            ctx,
            addr,
            buf: Vec::new(),
        })
    }

    /// Create a node by name.
    pub async fn create_node(&mut self, body: &CreateNode<'_>) -> ockam_core::Result<NodeInfo<'_>> {
        let req = Request::post("/", true);
        trace!(id = %req.id(), name = %body.name(), "creating new node");
        self.buf = self.request("create-node", &req, Some(body)).await?;
        let mut d = self.response("create-node")?;
        d.decode().map_err(|e| e.into())
    }

    /// Get information about a node.
    pub async fn get(&mut self, id: &str) -> ockam_core::Result<NodeInfo<'_>> {
        let req = Request::get(format!("/{id}"), false);
        trace!(id = %req.id(), node = %id, "getting node info");
        self.buf = self.request("get-node", &req, None::<()>).await?;
        let mut d = self.response("get-node")?;
        d.decode().map_err(|e| e.into())
    }

    /// List all available nodes.
    ///
    /// TODO: paging
    pub async fn list(&mut self) -> ockam_core::Result<Vec<NodeInfo<'_>>> {
        let req = Request::get("/", false);
        trace!(id = %req.id(), "listing all nodes");
        self.buf = self.request("list-nodes", &req, None::<()>).await?;
        let mut d = self.response("list-nodes")?;
        d.decode().map_err(|e| e.into())
    }

    /// Encode request header and body (if any) and send the package to the server.
    async fn request<T>(
        &mut self,
        label: &str,
        hdr: &Request<'_>,
        body: Option<T>,
    ) -> ockam_core::Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        assert_eq!(body.is_some(), hdr.has_body());

        let req = {
            let mut e = Encoder::new(Vec::new());
            e.encode(&hdr)?;
            if let Some(b) = body {
                e.encode(b)?;
            }
            e.into_writer()
        };

        trace!(label = %label, id = %hdr.id(), "-> req");

        let vec: Vec<u8> = self.ctx.send_and_receive(self.addr.clone(), req).await?;
        Ok(vec)
    }

    /// Receive the server response and check for errors.
    ///
    /// Returns the decoder positioned at the start of the response body.
    fn response<'b>(&'b mut self, label: &str) -> ockam_core::Result<Decoder<'b>> {
        let mut dec = Decoder::new(&self.buf);
        let res: Response = dec.decode()?;

        if res.status() == Some(Status::Ok) {
            trace! {
                label  = %label,
                id     = %res.id(),
                re     = %res.re(),
                status = ?res.status(),
                body   = %res.has_body(),
                "<- res"
            }
            return Ok(dec);
        }

        if res.has_body() {
            let err = dec.decode::<Error>()?;
            warn! {
                label  = %label,
                id     = %res.id(),
                re     = %res.re(),
                status = ?res.status(),
                error  = ?err.message(),
                "<- err"
            }
            let msg = err.message().unwrap_or(label);
            return Err(ockam_core::Error::new(
                Origin::Application,
                Kind::Protocol,
                msg,
            ));
        }

        return Err(ockam_core::Error::new(
            Origin::Application,
            Kind::Protocol,
            label,
        ));
    }
}

/// Potential node errors.
#[derive(Debug)]
pub struct NodesError(ErrorImpl);

#[derive(Debug)]
enum ErrorImpl {
    Decode(minicbor::decode::Error),
    Encode(minicbor::encode::Error<io::Error>),
}

impl fmt::Display for NodesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Encode(e) => e.fmt(f),
            ErrorImpl::Decode(e) => e.fmt(f),
        }
    }
}

impl ockam_core::compat::error::Error for NodesError {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::Decode(e) => Some(e),
            ErrorImpl::Encode(e) => Some(e),
        }
    }
}

impl From<minicbor::decode::Error> for NodesError {
    fn from(e: minicbor::decode::Error) -> Self {
        NodesError(ErrorImpl::Decode(e))
    }
}

impl From<minicbor::encode::Error<io::Error>> for NodesError {
    fn from(e: minicbor::encode::Error<io::Error>) -> Self {
        NodesError(ErrorImpl::Encode(e))
    }
}

impl From<NodesError> for ockam_core::Error {
    fn from(e: NodesError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e)
    }
}
