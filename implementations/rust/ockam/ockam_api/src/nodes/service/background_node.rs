use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::local_multiaddr_to_route;
use crate::nodes::NODEMANAGER_ADDR;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decode, Encode};
use ockam_core::api::{Reply, Request, Response, Status};
use ockam_core::{AsyncTryClone, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_transport_tcp::{TcpConnectionOptions, TcpTransport};
use std::sync::Arc;
use std::time::Duration;

/// This struct represents a node that has been started
/// on the same machine with a given node name
#[derive(Clone)]
pub struct BackgroundNode {
    cli_state: CliState,
    node_name: String,
    to: Route,
    timeout: Option<Duration>,
    tcp_transport: Arc<TcpTransport>,
}

impl BackgroundNode {
    /// Create a new client to send requests to a running background node
    /// This function instantiates a TcpTransport. Since a TcpTransport can only be created once
    /// this function must only be called once
    pub async fn create(
        ctx: &Context,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNode> {
        let tcp_transport = TcpTransport::create(ctx).await.into_diagnostic()?;
        BackgroundNode::new(&tcp_transport, cli_state, node_name).await
    }

    /// Create a new client to send requests to a running background node
    pub async fn new(
        tcp_transport: &TcpTransport,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNode> {
        Ok(BackgroundNode {
            cli_state: cli_state.clone(),
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            tcp_transport: Arc::new(tcp_transport.async_try_clone().await.into_diagnostic()?),
        })
    }

    pub fn node_name(&self) -> &str {
        &self.node_name
    }

    pub fn set_node_name(&mut self, node_name: &str) -> &Self {
        self.node_name = node_name.to_string();
        self
    }

    /// Use a timeout for making requests
    pub fn set_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    /// Use a timeout for making requests
    pub fn with_timeout(&self, timeout: Option<Duration>) -> Self {
        Self {
            cli_state: self.cli_state.clone(),
            node_name: self.node_name.clone(),
            to: self.to.clone(),
            timeout,
            tcp_transport: self.tcp_transport.clone(),
        }
    }

    pub fn set_to(&mut self, to: &MultiAddr) -> miette::Result<&Self> {
        self.to = local_multiaddr_to_route(to).into_diagnostic()?;
        Ok(self)
    }

    /// Send a request
    /// This method waits for a response status but does not expect a response body
    /// If the status is missing or not, we try to parse an error message and return it
    pub async fn tell<T>(&self, ctx: &Context, req: Request<T>) -> miette::Result<()>
    where
        T: Encode<()>,
    {
        let bytes = self.send_request(ctx, req).await?;
        let (response, decoder) =
            Response::parse_response_header(bytes.as_slice()).into_diagnostic()?;
        if !response.is_ok() {
            Err(miette!(response.parse_err_msg(decoder)))
        } else {
            Ok(())
        }
    }

    /// Send a request
    /// This method waits for a response status and returns it if available
    pub async fn tell_and_get_status<T>(
        &self,
        ctx: &Context,
        req: Request<T>,
    ) -> miette::Result<Option<Status>>
    where
        T: Encode<()>,
    {
        self.parse_response_status(self.send_request(ctx, req).await?)
    }

    /// Send a request and expects a decodable response
    /// This method parses and returns an error message if the request was not successful
    pub async fn ask<T, R>(&self, ctx: &Context, req: Request<T>) -> miette::Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.parse_response_body::<R>(self.send_request(ctx, req).await?)
    }

    /// Send a request and expects either a decodable response or an API error.
    /// This method returns an error if the request cannot be sent of if there is any decoding error
    pub async fn ask_and_get_reply<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
    ) -> miette::Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.parse_response_reply::<R>(self.send_request(ctx, req).await?)
    }

    /// Make a request and wait for a response
    /// This method _does not_ check the success of the request
    async fn send_request<T>(&self, ctx: &Context, req: Request<T>) -> miette::Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let route = self.route_impl().await?;
        let options = self
            .timeout
            .map(|t| MessageSendReceiveOptions::new().with_timeout(t))
            .unwrap_or(MessageSendReceiveOptions::new());
        Ok(ctx
            .send_and_receive_extended::<Vec<u8>>(
                route.clone(),
                req.to_vec().into_diagnostic()?,
                options,
            )
            .await
            .into_diagnostic()?
            .body())
    }

    async fn route_impl(&self) -> miette::Result<Route> {
        let mut route = self.to.clone();
        let node_state = self.cli_state.nodes.get(&self.node_name)?;
        let port = node_state.config().setup().api_transport()?.addr.port();
        let addr_str = format!("localhost:{port}");
        let addr = self
            .tcp_transport
            .connect(addr_str, TcpConnectionOptions::new())
            .await
            .into_diagnostic()?
            .sender_address()
            .clone();
        route.modify().prepend(addr);
        debug!(%route, "Sending request");
        Ok(route)
    }

    /// Parse the response body and return it
    /// This function returns an Err with a parsed error message if the response status is not ok
    fn parse_response_body<T>(&self, bytes: Vec<u8>) -> miette::Result<T>
    where
        T: for<'b> Decode<'b, ()>,
    {
        Response::parse_response_body(bytes.as_slice()).into_diagnostic()
    }

    /// Parse the response body and return it
    /// This function returns an Err with a parsed error message if the response status is not ok
    fn parse_response_reply<T>(&self, bytes: Vec<u8>) -> miette::Result<Reply<T>>
    where
        T: for<'b> Decode<'b, ()>,
    {
        Response::parse_response_reply(bytes.as_slice()).map_err(|e| miette!(e))
    }

    /// Parse a Response and return its status
    fn parse_response_status(&self, bytes: Vec<u8>) -> miette::Result<Option<Status>> {
        let (response, _decoder) =
            Response::parse_response_header(bytes.as_slice()).into_diagnostic()?;
        Ok(response.status())
    }
}
