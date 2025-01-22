use crate::control_api::{build_error_body, ControlApiHttpRequest, ControlApiHttpResponse};
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use hyper_util::rt::TokioIo;
use ockam_abac::{IncomingAbac, OutgoingAbac, PolicyExpression};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    async_trait, cbor_encode_preallocate, route, Address, AllowAll, Error, IncomingAccessControl,
    NeutralMessage, OutgoingAccessControl, Processor, Routed, TryClone,
};
use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;

const MAX_REQUEST_SIZE: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

pub struct HttpControlNodeApiFrontend {
    listener: TcpListener,
    authentication_token: String,
    context: Option<Arc<Context>>,
    incoming_access_control: Arc<dyn IncomingAccessControl>,
    outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub node_resolution: NodeResolution,
}

#[async_trait]
impl Processor for HttpControlNodeApiFrontend {
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> ockam_core::Result<()> {
        // We don't need to clone the whole context each time since send_and_receive already
        // creates dedicated addresses.
        self.context = Some(Arc::new(context.try_clone()?));
        Ok(())
    }

    async fn process(&mut self, _context: &mut Self::Context) -> ockam_core::Result<bool> {
        let context = match self.context.as_ref() {
            Some(context) => context.clone(),
            None => {
                error!("Context not initialized");
                return Ok(false);
            }
        };

        let (stream, _from) = match self.listener.accept().await {
            Ok(data) => data,
            Err(error) => {
                error!("Cannot accept incoming connection {:?}", error);
                return Ok(false);
            }
        };

        let service = service_fn(|request| {
            let context = context.clone();
            Self::route_request(
                request,
                context,
                self.node_resolution.clone(),
                self.authentication_token.clone(),
                self.incoming_access_control.clone(),
                self.outgoing_access_control.clone(),
            )
        });

        if let Err(err) = http1::Builder::new()
            .serve_connection(TokioIo::new(stream), service)
            .await
        {
            error!("HTTP server error: {:?}", err);
        }

        Ok(true)
    }
}

impl HttpControlNodeApiFrontend {
    async fn new(
        bind_address: SocketAddr,
        authentication_token: String,
        node_resolution: NodeResolution,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> ockam_core::Result<Self> {
        let listener = TcpListener::bind(bind_address)
            .await
            .map_err(TransportError::from)?;

        Ok(Self {
            listener,
            authentication_token,
            context: None,
            node_resolution,
            incoming_access_control,
            outgoing_access_control,
        })
    }

    fn bind_address(&self) -> ockam_core::Result<SocketAddr> {
        Ok(self.listener.local_addr().map_err(TransportError::from)?)
    }

    async fn route_request(
        request: Request<IncomingBody>,
        context: Arc<Context>,
        node_resolution: NodeResolution,
        authentication_token: String,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> http::Result<Response<Full<Bytes>>> {
        match Self::route_request_impl(
            request,
            context,
            node_resolution,
            authentication_token,
            incoming_access_control,
            outgoing_access_control,
        )
        .await
        {
            Ok(response) => Ok(response),
            Err(error) => {
                error!("Error processing request: {:?}", error);
                let result = match error.code().kind {
                    Kind::Timeout => Response::builder()
                        .status(504)
                        .body(build_error_body("Request timed out")),
                    _ => Response::builder()
                        .status(500)
                        .body(build_error_body(&error.to_string())),
                };

                match result {
                    Ok(response) => Ok(response),
                    Err(encoding_error) => {
                        error!("Error encoding error response: {:?}", encoding_error);
                        Response::builder().status(500).body(Full::default())
                    }
                }
            }
        }
    }

    async fn route_request_impl(
        request: Request<IncomingBody>,
        context: Arc<Context>,
        node_resolution: NodeResolution,
        authentication_token: String,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        if let Some(response) = Self::authenticate_request(&request, authentication_token)? {
            return Ok(response);
        }

        let uri = request.uri().clone();

        let path = uri.path();
        let first_slash = match path[1..].find('/') {
            Some(index) => index,
            None => {
                error!("Received request with invalid path: {}", path);
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    "Missing node name in request path",
                ));
            }
        };

        let method = request.method().to_string();
        let node_name = &path[1..first_slash + 1];
        debug!("Received request for node: {}", node_name);

        let body = http_body_util::Limited::new(request.into_body(), MAX_REQUEST_SIZE);
        let serialized_body = body
            .collect()
            .await
            .map_err(|_| Error::new(Origin::Api, Kind::Io, "Failed to read request body"))?;

        let body = serialized_body.to_bytes();
        let body = if body.is_empty() {
            None
        } else {
            Some(body.to_vec())
        };

        let control_api_request = ControlApiHttpRequest {
            method,
            uri: uri.to_string(),
            body,
        };

        let message = cbor_encode_preallocate(&control_api_request)
            .map_err(|_| Error::new(Origin::Api, Kind::Internal, "Failed to encode request"))?;

        let node_connection;
        let node_route = match &node_resolution {
            NodeResolution::Relay => {
                node_connection = None;
                route![
                    Address::from_string(format!("forward_to_{node_name}")),
                    DefaultAddress::CONTROL_API
                ]
            }
            NodeResolution::DirectConnection {
                node_manager,
                suffix,
                port,
            } => {
                let result = node_manager
                    .make_connection(
                        &context,
                        &format!(
                            "/dnsaddr/{node_name}{suffix}/tcp/{port}/secure/api/service/{}",
                            DefaultAddress::CONTROL_API
                        )
                        .parse()?,
                        node_manager.identifier(),
                        None,
                        None,
                    )
                    .await;

                match result {
                    Ok(connection) => {
                        let route = connection.route()?;
                        node_connection = Some(connection);
                        route
                    }
                    Err(error) => {
                        error!("Failed to create connection to node: {:?}", error);
                        return Response::builder()
                            .status(502)
                            .body(build_error_body("Node not reachable"))
                            .map_err(Self::map_http_err);
                    }
                }
            }
        };

        let result: ockam_core::Result<Routed<NeutralMessage>> = context
            .send_and_receive_extended(
                node_route,
                NeutralMessage::from(message),
                MessageSendReceiveOptions::new()
                    .with_timeout(TIMEOUT)
                    .with_incoming_access_control(incoming_access_control)
                    .with_outgoing_access_control(outgoing_access_control),
            )
            .await;

        // close the connection regardless of the result
        if let NodeResolution::DirectConnection { node_manager, .. } = node_resolution {
            if let Some(connection) = node_connection {
                connection.close(&context, &node_manager)?;
            }
        }

        let response = match result {
            Ok(response) => response,
            Err(error) => {
                return match error.code().kind {
                    Kind::NotFound => Response::builder()
                        .status(502)
                        .body(build_error_body("Node not reachable"))
                        .map_err(Self::map_http_err),
                    _ => Response::builder()
                        .status(500)
                        .body(build_error_body(&error.to_string()))
                        .map_err(Self::map_http_err),
                }
            }
        };

        let response: ControlApiHttpResponse = minicbor::decode(response.payload())
            .map_err(|_| Error::new(Origin::Api, Kind::Protocol, "Failed to decode response"))?;

        let body = Full::from(Bytes::from(response.body));

        Response::builder()
            .status(response.status)
            .body(body)
            .map_err(Self::map_http_err)
    }

    fn map_http_err(_error: http::Error) -> ockam_core::Error {
        Error::new(Origin::Api, Kind::Internal, "Failed to build response")
    }

    fn authenticate_request(
        request: &Request<Incoming>,
        authentication_token: String,
    ) -> ockam_core::Result<Option<Response<Full<Bytes>>>> {
        let headers = request.headers();
        let auth_header = match headers.get("Authorization") {
            Some(header) => header,
            None => {
                return Ok(Some(
                    Response::builder()
                        .status(401)
                        .body(build_error_body("Missing authentication token"))
                        .map_err(Self::map_http_err)?,
                ))
            }
        };

        let authentication_header = match auth_header.to_str() {
            Ok(header) => header,
            Err(_) => {
                return Ok(Some(
                    Response::builder()
                        .status(401)
                        .body(build_error_body("Invalid authentication token"))
                        .map_err(Self::map_http_err)?,
                ))
            }
        };

        // extract authentication token from header
        let auth_header = authentication_header
            .split_whitespace()
            .last()
            .unwrap_or_default();

        // constant time comparison, to prevent timing attacks
        let authenticated: bool = authentication_token
            .as_bytes()
            .ct_eq(auth_header.as_bytes())
            .into();

        if authenticated {
            Ok(None)
        } else {
            Ok(Some(
                Response::builder()
                    .status(401)
                    .body(build_error_body("Invalid authentication token"))
                    .map_err(Self::map_http_err)?,
            ))
        }
    }
}

#[derive(Clone)]
pub enum NodeResolution {
    Relay,
    DirectConnection {
        node_manager: Arc<NodeManager>,
        suffix: String,
        port: u16,
    },
}

impl NodeManager {
    pub async fn create_control_api_frontend(
        &self,
        context: &Context,
        bind_address: SocketAddr,
        node_resolution: NodeResolution,
        authentication_token: String,
        policy_expression: Option<PolicyExpression>,
    ) -> ockam_core::Result<SocketAddr> {
        let incoming_access_control: Arc<dyn IncomingAccessControl>;
        let outgoing_access_control: Arc<dyn OutgoingAccessControl>;

        match policy_expression {
            Some(policy_expression) => {
                incoming_access_control = Arc::new(IncomingAbac::create(
                    self.secure_channels.identities().identities_attributes(),
                    self.project_authority(),
                    policy_expression.to_expression(),
                ));
                outgoing_access_control = Arc::new(OutgoingAbac::create(
                    context,
                    self.secure_channels.identities().identities_attributes(),
                    self.project_authority(),
                    policy_expression.to_expression(),
                )?);
            }
            None => {
                incoming_access_control = Arc::new(AllowAll);
                outgoing_access_control = Arc::new(AllowAll);
            }
        }

        let frontend = HttpControlNodeApiFrontend::new(
            bind_address,
            authentication_token,
            node_resolution,
            incoming_access_control,
            outgoing_access_control,
        )
        .await?;
        let bind_address = frontend.bind_address()?;
        context.start_processor("http_control_node_api_frontend", frontend)?;
        Ok(bind_address)
    }
}

#[cfg(test)]
mod test {
    use crate::control_api::frontend::NodeResolution;
    use crate::control_api::protocol::inlet::InletStatus;
    use crate::control_api::ErrorResponse;
    use crate::hop::Hop;
    use crate::test_utils::start_manager_for_tests;
    use bytes::Bytes;
    use colorful::core::StrMarker;
    use http::{Request, Response};
    use http_body_util::{BodyExt, Full};
    use hyper_util::rt::TokioIo;
    use ockam_node::Context;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::net::SocketAddr;
    use tokio::net::TcpStream;

    async fn send_http_request<'a, S: Serialize + Send, D: DeserializeOwned + Send>(
        request: Request<S>,
    ) -> Response<D> {
        let host = request.uri().host().expect("uri has no host");
        let port = request.uri().port().expect("uri has no port");

        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(addr).await.unwrap();
        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
        tokio::task::spawn(async move {
            let _ = conn.await;
        });

        // Serialize the body and recreate the request
        let (parts, body) = request.into_parts();
        let body: Full<Bytes> = Full::from(Bytes::from(serde_json::to_vec(&body).unwrap()));
        let request = Request::from_parts(parts, body);

        let response = sender.send_request(request).await.unwrap();

        // Deserialize the body and recreate the response
        let (parts, body) = response.into_parts();
        let serialized_body = body.collect().await.unwrap();
        let bytes = serialized_body.to_bytes();
        let body = serde_json::from_slice(bytes.as_ref()).unwrap();

        Response::from_parts(parts, body)
    }

    #[ockam::test]
    pub async fn authentication_token_validation(context: &mut Context) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;

        let bind_address = handle
            .node_manager
            .create_control_api_frontend(
                context,
                SocketAddr::from(([127, 0, 0, 1], 0)),
                NodeResolution::Relay,
                "token".to_str(),
                None,
            )
            .await?;

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 401, "Unauthorized");
        assert_eq!(response.body().message, "Missing authentication token");

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "invalid_token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 401, "Unauthorized");
        assert_eq!(response.body().message, "Invalid authentication token");

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "Bearer invalid_token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 401, "Unauthorized");
        assert_eq!(response.body().message, "Invalid authentication token");

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "Bearer token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 502);

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 502);

        Ok(())
    }

    #[ockam::test]
    pub async fn reverse_proxy_using_relay(context: &mut Context) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;

        let bind_address = handle
            .node_manager
            .create_control_api_frontend(
                context,
                SocketAddr::from(([127, 0, 0, 1], 0)),
                NodeResolution::Relay,
                "token".to_str(),
                None,
            )
            .await?;

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        context.start_worker("forward_to_node1", Hop)?;

        let response: Response<Vec<InletStatus>> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "Bearer token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 200);

        Ok(())
    }

    #[ockam::test]
    pub async fn reverse_proxy_using_relay_non_existing_node(
        context: &mut Context,
    ) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;

        let bind_address = handle
            .node_manager
            .create_control_api_frontend(
                context,
                SocketAddr::from(([127, 0, 0, 1], 0)),
                NodeResolution::Relay,
                "token".to_str(),
                None,
            )
            .await?;

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!(
                "http://{bind_address}/non-existing-node/tcp-inlet/"
            ))
            .header("Authorization", "Bearer token")
            .body(())
            .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 502, "the node is not reachable");
        assert_eq!(response.body().message, "Node not reachable");

        Ok(())
    }

    #[ockam::test]
    pub async fn reverse_proxy_using_direct_connection(
        context: &mut Context,
    ) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;

        let bind_address = handle
            .node_manager
            .create_control_api_frontend(
                context,
                SocketAddr::from(([127, 0, 0, 1], 0)),
                NodeResolution::DirectConnection {
                    node_manager: handle.node_manager.inner_clone(),
                    suffix: ".localhost".to_string(),
                    port: handle.bind_address.port(),
                },
                "token".to_str(),
                None,
            )
            .await?;

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        let response: Response<Vec<InletStatus>> = send_http_request(
            Request::get(format!("http://{bind_address}/node1/tcp-inlet"))
                .header("Authorization", "Bearer token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 200);

        Ok(())
    }

    #[ockam::test]
    pub async fn reverse_proxy_using_direct_connection_non_existing_node(
        context: &mut Context,
    ) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;

        let bind_address = handle
            .node_manager
            .create_control_api_frontend(
                context,
                SocketAddr::from(([127, 0, 0, 1], 0)),
                NodeResolution::DirectConnection {
                    node_manager: handle.node_manager.inner_clone(),
                    suffix: ".localhost".to_string(),
                    port: 0,
                },
                "token".to_str(),
                None,
            )
            .await?;

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        let response: Response<ErrorResponse> = send_http_request(
            Request::get(format!("http://{bind_address}/non-existing-node/tcp-inlet"))
                .header("Authorization", "Bearer token")
                .body(())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status().as_u16(), 502, "the node is not reachable");
        assert_eq!(response.body().message, "Node not reachable");

        Ok(())
    }
}
