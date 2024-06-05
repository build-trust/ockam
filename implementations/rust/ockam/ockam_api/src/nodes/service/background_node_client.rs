use std::sync::Arc;
use std::time::Duration;

use miette::{miette, IntoDiagnostic};
use minicbor::{Decode, Encode};

use ockam::tcp::{TcpConnection, TcpConnectionOptions, TcpTransport};
use ockam_core::api::{Reply, Request};
use ockam_core::Route;
use ockam_node::api::Client;
use ockam_node::Context;

use crate::cli_state::CliState;
use crate::nodes::NODEMANAGER_ADDR;

/// This struct represents a Client to a node that has been started
/// on the same machine with a given node name
///
/// The methods on this struct allow a user to send requests containing a value of type `T`
/// and expect responses with a value of type `R`
#[derive(Clone)]
pub struct BackgroundNodeClient {
    cli_state: CliState,
    node_name: String,
    to: Route,
    timeout: Option<Duration>,
    tcp_transport: Arc<TcpTransport>,
}

impl BackgroundNodeClient {
    /// Create a new client to send requests to a running background node
    /// This function instantiates a TcpTransport. Since a TcpTransport can only be created once
    /// this function must only be called once
    ///
    /// The optional node name is used to locate the node. It is either
    /// a node specified by the user or the default node if no node name is given.
    pub async fn create(
        ctx: &Context,
        cli_state: &CliState,
        node_name: &Option<String>,
    ) -> miette::Result<BackgroundNodeClient> {
        let node_name = match node_name.clone() {
            Some(name) => name,
            None => cli_state.get_default_node().await?.name(),
        };
        Self::create_to_node(ctx, cli_state, &node_name).await
    }

    pub async fn create_to_node(
        ctx: &Context,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNodeClient> {
        let tcp_transport = TcpTransport::create(ctx).await.into_diagnostic()?;
        BackgroundNodeClient::new(&tcp_transport, cli_state, node_name)
    }

    pub async fn create_to_node_with_tcp(
        tcp: &TcpTransport,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNodeClient> {
        BackgroundNodeClient::new(tcp, cli_state, node_name)
    }

    /// Create a new client to send requests to a running background node
    pub fn new(
        tcp_transport: &TcpTransport,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNodeClient> {
        Ok(BackgroundNodeClient {
            cli_state: cli_state.clone(),
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            timeout: Some(Duration::from_secs(30)),
            tcp_transport: Arc::new(tcp_transport.clone()),
        })
    }

    pub async fn delete(&self) -> miette::Result<()> {
        Ok(self.cli_state.delete_node(&self.node_name(), false).await?)
    }

    // Set a different node name
    pub fn set_node_name(&mut self, node_name: &str) -> &Self {
        self.node_name = node_name.to_string();
        self
    }

    pub fn node_name(&self) -> String {
        self.node_name.clone()
    }

    /// Use a default timeout for making requests
    pub fn set_timeout_mut(&mut self, timeout: Duration) -> &Self {
        self.timeout = Some(timeout);
        self
    }

    /// Use a default timeout for making requests
    pub fn set_timeout(self, timeout: Option<Duration>) -> Self {
        Self { timeout, ..self }
    }

    pub fn cli_state(&self) -> &CliState {
        &self.cli_state
    }

    /// Send a request and expect a decodable response
    pub async fn ask<T, R>(&self, ctx: &Context, req: Request<T>) -> miette::Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.ask_and_get_reply(ctx, req)
            .await?
            .success()
            .into_diagnostic()
    }

    /// Send a request and expect a decodable response and use a specific timeout
    pub async fn ask_with_timeout<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Duration,
    ) -> miette::Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        let (tcp_connection, client) = self.make_client_with_timeout(Some(timeout)).await?;

        let res = client
            .ask(ctx, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic();

        _ = tcp_connection.stop(ctx).await;
        res
    }

    /// Send a request and expect either a decodable response or an API error.
    /// This method returns an error if the request cannot be sent or if there is any decoding error
    pub async fn ask_and_get_reply<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
    ) -> miette::Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        let (tcp_connection, client) = self.make_client().await?;
        let res = client.ask(ctx, req).await.into_diagnostic();

        _ = tcp_connection.stop(ctx).await;
        res
    }

    /// Send a request but don't decode the response
    pub async fn tell<T>(&self, ctx: &Context, req: Request<T>) -> miette::Result<()>
    where
        T: Encode<()>,
    {
        let (tcp_connection, client) = self.make_client().await?;
        let res = client
            .tell(ctx, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic();

        _ = tcp_connection.stop(ctx).await;
        res
    }

    /// Send a request but and return the API reply without decoding the body response
    pub async fn tell_and_get_reply<T>(
        &self,
        ctx: &Context,
        req: Request<T>,
    ) -> miette::Result<Reply<()>>
    where
        T: Encode<()>,
    {
        let (tcp_connection, client) = self.make_client().await?;
        let res = client.tell(ctx, req).await.into_diagnostic();

        _ = tcp_connection.stop(ctx).await;
        res
    }

    /// This method succeeds if a TCP connection can be established with the node
    pub async fn is_accessible(&self, ctx: &Context) -> miette::Result<()> {
        self.create_tcp_connection()
            .await?
            .stop(ctx)
            .await
            .into_diagnostic()
    }

    /// Make a route to the node and connect using TCP
    async fn create_route(&self) -> miette::Result<(TcpConnection, Route)> {
        let tcp_connection = self.create_tcp_connection().await?;
        let mut route = self.to.clone();
        route
            .modify()
            .prepend(tcp_connection.sender_address().clone());
        debug!("Sending requests to {route}");
        Ok((tcp_connection, route))
    }

    /// Create a TCP connection to the node
    async fn create_tcp_connection(&self) -> miette::Result<TcpConnection> {
        let node_info = self.cli_state.get_node(&self.node_name).await?;
        let tcp_listener_address = node_info
            .tcp_listener_address()
            .ok_or(miette!(
                "an api transport should have been started for node {:?}",
                &node_info
            ))?
            .to_string();

        self.tcp_transport
            .connect(&tcp_listener_address, TcpConnectionOptions::new())
            .await
            .map_err(|_| {
                miette!(
                    "Failed to connect to node {} at {}",
                    &self.node_name,
                    &tcp_listener_address
                )
            })
    }

    /// Make a response / request client connected to the node
    pub(crate) async fn make_client(&self) -> miette::Result<(TcpConnection, Client)> {
        self.make_client_with_timeout(self.timeout).await
    }

    /// Make a response / request client connected to the node
    /// and specify a timeout for receiving responses
    pub(crate) async fn make_client_with_timeout(
        &self,
        timeout: Option<Duration>,
    ) -> miette::Result<(TcpConnection, Client)> {
        let (tcp_connection, route) = self.create_route().await?;
        Ok((tcp_connection, Client::new(&route, timeout)))
    }
}
