//! Node Manager (Node Man, the superhero that we deserve)

use crate::{
    nodes::types::{NodeStatus, TransportList, TransportMode, TransportStatus, TransportType},
    Method, Request, Response, ResponseBuilder,
};
use minicbor::{encode::Write, Decoder};
use ockam::{Context, Result, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, io, string::String};

use super::types::CreateTransport;

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan {
    node_name: String,
    transports: Vec<(TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,
}

impl NodeMan {
    /// Create a new NodeMan with the node name from the ockam CLI
    pub fn new(
        node_name: String,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Self {
        Self {
            node_name,
            transports: vec![api_transport],
            tcp_transport,
        }
    }
}

impl NodeMan {
    fn get_transports(&self) -> Vec<TransportStatus<'_>> {
        self.transports
            .iter()
            .map(|(tt, tm, addr)| TransportStatus::new(*tt, *tm, addr))
            .collect()
    }

    async fn add_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'_>>> {
        let CreateTransport { tt, tm, addr, .. } = dec.decode()?;

        use {TransportMode::*, TransportType::*};

        info!(
            "Handling request to create a new transport: {}, {}, {}",
            tt, tm, addr
        );

        let res = match (tt, tm) {
            (Tcp, Listen) => self
                .tcp_transport
                .listen(addr)
                .await
                .map(|socket| socket.to_string()),
            (Tcp, Connect) => self
                .tcp_transport
                .connect(addr)
                .await
                .map(|ockam_addr| ockam_addr.to_string()),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok(addr) => {
                self.transports.push((tt, tm, addr.clone()));
                Response::ok(req.id()).body(TransportStatus::new(tt, tm, addr))
            }
            Err(msg) => {
                Response::bad_request(req.id()).body(TransportStatus::new(tt, tm, msg.to_string()))
            }
        };

        Ok(response)
    }

    async fn handle_request<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = io::Error>,
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
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        match (method, path) {
            // == Get information about this node
            (Get, "/node") => Response::ok(req.id())
                .body(NodeStatus::new(
                    self.node_name.as_str(),
                    "[✓]",
                    ctx.list_workers().await?.len() as u32,
                    std::process::id() as i32,
                    self.transports.len() as u32,
                ))
                .encode(enc)?,
            // == Get all transports
            (Get, "/node/transport") => Response::ok(req.id())
                .body(TransportList::new(self.get_transports()))
                .encode(enc)?,
            // TODO: Get all transports
            // == Create a new transport
            (Post, "/node/transport") => self.add_transport(req, dec).await?.encode(enc)?,
            (method, path) => {
                warn!("Called invalid endpoint: {} {}", method, path);
                todo!()
            }
        }

        Ok(())
    }
}

#[ockam::worker]
impl Worker for NodeMan {
    type Message = Vec<u8>;
    type Context = Context;

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

        self.handle_request(ctx, &req, &mut dec, &mut buf).await?;
        ctx.send(msg.return_route(), buf).await
    }
}
