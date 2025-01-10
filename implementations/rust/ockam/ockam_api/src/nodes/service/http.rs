use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Weak};

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;

use crate::nodes::models::transport::Port;
use crate::nodes::NodeManager;
use crate::{ApiError, HttpError, Result};
use ockam_core::{async_trait, Address, Processor};
use ockam_node::{Context, ProcessorBuilder};
use serde::Serialize;
use tokio::net::TcpListener;

/// An HTTP server that provides health check endpoints for the node.
///
/// This server is complementary to the node's API and is intended to be used
/// for health checks and monitoring of the node's status.
///
/// It is not intended to be a full-fledged HTTP version of the node's API.
pub struct HttpServer;

impl HttpServer {
    /// Start a new HTTP server listening on the given port
    /// and return a handle to it that will be used to cancel the
    /// background async task when the NodeManager shuts down.
    pub async fn start(
        context: &Context,
        node_manager: Arc<NodeManager>,
        port: Port,
    ) -> Result<SocketAddr> {
        debug!("Starting HTTP server on port: {port:?}");
        let listener = port.bind_to_tcp_listener().await?;
        let addr = listener.local_addr()?;
        node_manager
            .cli_state
            .set_node_http_server_addr(&node_manager.node_name, &addr.into())
            .await?;
        let processor = HttpServerProcessor {
            node_manager: Arc::downgrade(&node_manager),
            tcp_listener: Arc::new(listener),
        };
        ProcessorBuilder::new(processor)
            .with_address(Address::random_tagged("node_http_server"))
            .start(context)?;
        info!("HTTP server listening on: {addr:?}");
        Ok(addr)
    }
}

struct HttpServerProcessor {
    node_manager: Weak<NodeManager>,
    tcp_listener: Arc<TcpListener>,
}

impl HttpServerProcessor {
    async fn handle_request(
        node_manager: Weak<NodeManager>,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>> {
        debug!("Processing request: {req:?}");
        let path = req
            .uri()
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        match (req.method(), path.as_slice()) {
            (&Method::HEAD, []) => Ok(Response::new(Full::new(Bytes::new()).boxed())),
            (&Method::GET, ["show"]) => {
                let node_resources = {
                    let node_manager = node_manager
                        .upgrade()
                        .ok_or_else(|| ApiError::core("node manager was shut down"))?;
                    node_manager.get_node_resources().await?
                };
                Self::json_response(node_resources)
            }
            _ => {
                warn!("Request received for a non supported endpoint: {req:?}");
                Ok(Response::builder()
                    .status(404)
                    .body(Empty::<Bytes>::new().boxed())
                    .map_err(HttpError::from)?)
            }
        }
    }

    fn json_response<T: Serialize>(data: T) -> Result<Response<BoxBody<Bytes, Infallible>>> {
        match serde_json::to_string(&data) {
            Ok(json) => Ok(Response::new(Full::new(Bytes::from(json)).boxed())),
            Err(err) => {
                error!("Error serializing response: {err:?}");
                let json = serde_json::json!({
                    "error": "failed to serialize response",
                })
                .to_string();
                Ok(Response::new(Full::new(Bytes::from(json)).boxed()))
            }
        }
    }
}

#[async_trait]
impl Processor for HttpServerProcessor {
    type Context = Context;

    async fn shutdown(&mut self, _context: &mut Self::Context) -> ockam_core::Result<()> {
        debug!("Shutting down HttpServerProcessor");
        Ok(())
    }

    async fn process(&mut self, _context: &mut Self::Context) -> ockam_core::Result<bool> {
        if let Ok((stream, _)) = self.tcp_listener.accept().await {
            let io = TokioIo::new(stream);
            let service = service_fn(|req| {
                let node_manager = self.node_manager.clone();
                Self::handle_request(node_manager, req)
            });
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Error serving connection: {err:?}");
            }
        }
        Ok(true)
    }
}
