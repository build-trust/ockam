use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;

use serde::Serialize;
use tokio::net::TcpListener;

use crate::nodes::NodeManager;
use crate::{HttpError, Result};

/// An HTTP server that provides health check endpoints for the node.
///
/// This server is complementary to the node's API and is intended to be used
/// for health checks and monitoring of the node's status.
///
/// It is not intended to be a full-fledged HTTP version of the node's API.
pub struct HttpServer {
    node_manager: Arc<NodeManager>,
}

impl HttpServer {
    /// Start a new HTTP server listening on the given port
    /// and return a handle to it that will be used to cancel the
    /// background async task when the NodeManager shuts down.
    pub async fn start(node_manager: Arc<NodeManager>, port: u16) -> Result<SocketAddr> {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
        let addr = listener.local_addr()?;
        let node_name = node_manager.node_name.clone();
        let server = Self {
            node_manager: node_manager.clone(),
        };
        tokio::spawn(server.run(listener));
        node_manager
            .cli_state
            .set_node_http_server_addr(&node_name, &addr.into())
            .await?;
        info!("Http server listening on: {addr:?}");
        Ok(addr)
    }

    /// Loop to accept incoming connections and handle them
    async fn run(self, listener: TcpListener) -> Result<()> {
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let node_manager = self.node_manager.clone();
            let service = service_fn(move |req| {
                let node_manager = node_manager.clone();
                Self::handle_request(node_manager, req)
            });
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Error serving connection: {err:?}");
                }
            });
        }
    }

    async fn handle_request(
        node_manager: Arc<NodeManager>,
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
                Self::json_response(node_manager.get_node_resources().await?)
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
