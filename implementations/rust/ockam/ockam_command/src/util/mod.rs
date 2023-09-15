use core::time::Duration;
use std::sync::Arc;
use std::{
    net::{SocketAddr, TcpListener},
    path::Path,
    str::FromStr,
};

use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decode, Encode};
use tracing::{debug, error};

use ockam::{
    Address, Context, MessageSendReceiveOptions, NodeBuilder, Route, TcpConnectionOptions,
    TcpTransport,
};
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::config::lookup::{InternetAddress, LookupMeta};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Reply, Request, Response, Status};
use ockam_core::AsyncTryClone;
use ockam_core::DenyAll;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Space, Tcp};
use ockam_multiaddr::{
    proto::{self, Node},
    MultiAddr, Protocol,
};

use crate::node::util::{
    start_node_manager_worker, start_node_manager_worker_with_vault_and_identity,
};
use crate::util::api::TrustContextOpts;
use crate::{CommandGlobalOpts, Result};

pub mod api;
pub mod duration;
pub mod exitcode;
pub mod parsers;

#[derive(Clone)]
pub enum RpcMode {
    Embedded,
    Background(Arc<TcpTransport>),
}

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct Rpc {
    ctx: Context,
    buf: Vec<u8>,
    pub opts: CommandGlobalOpts,
    node_name: String,
    to: Route,
    pub timeout: Option<Duration>,
    mode: RpcMode,
}

impl Rpc {
    /// Creates a new RPC to send a request to an embedded node.
    pub async fn embedded(ctx: &Context, opts: &CommandGlobalOpts) -> Result<Rpc> {
        let node_name = start_node_manager_worker(ctx, opts, None).await?;
        let ctx_clone = ctx.async_try_clone().await?;
        Ok(Rpc {
            ctx: ctx_clone,
            buf: Vec::new(),
            opts: opts.clone(),
            node_name,
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            mode: RpcMode::Embedded,
        })
    }

    /// Creates a new RPC to send a request to an embedded node.
    pub async fn embedded_with_trust_options(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        trust_context_opts: &TrustContextOpts,
    ) -> Result<Rpc> {
        let node_name = start_node_manager_worker(ctx, opts, Some(trust_context_opts)).await?;
        let ctx_clone = ctx.async_try_clone().await?;
        Ok(Rpc {
            ctx: ctx_clone,
            buf: Vec::new(),
            opts: opts.clone(),
            node_name,
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            mode: RpcMode::Embedded,
        })
    }

    /// Creates a new RPC to send a request to an embedded node.
    pub async fn embedded_with_vault_and_identity(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        identity: String,
        trust_context_opts: &TrustContextOpts,
    ) -> Result<Rpc> {
        let node_name = start_node_manager_worker_with_vault_and_identity(
            ctx,
            &opts.state,
            None,
            Some(identity),
            Some(trust_context_opts),
        )
        .await?;

        let ctx_clone = ctx.async_try_clone().await?;
        Ok(Rpc {
            ctx: ctx_clone,
            buf: Vec::new(),
            opts: opts.clone(),
            node_name,
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            mode: RpcMode::Embedded,
        })
    }

    /// Creates a new RPC to send a request to a running background node.
    pub async fn background(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        node_name: &str,
    ) -> Result<Rpc> {
        let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
        let ctx_clone = ctx.async_try_clone().await?;
        Ok(Rpc {
            ctx: ctx_clone,
            buf: Vec::new(),
            opts: opts.clone(),
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            mode: RpcMode::Background(Arc::new(tcp)),
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

    pub fn set_to(&mut self, to: &MultiAddr) -> Result<&Self> {
        self.to = ockam_api::local_multiaddr_to_route(to)
            .ok_or_else(|| miette!("failed to convert {} to route", to))?;
        Ok(self)
    }

    /// Send a request
    /// This method waits for a response status but does not expect a response body
    /// If the status is missing or not, we try to parse an error message and return it
    pub async fn tell<T>(&mut self, req: Request<T>) -> Result<()>
    where
        T: Encode<()>,
    {
        self.send_request(req).await?;
        let (response, decoder) = Response::parse_response_header(self.buf.as_slice())?;
        if !response.is_ok() {
            Err(miette!(response.parse_err_msg(decoder)).into())
        } else {
            Ok(())
        }
    }

    /// Send a request
    /// This method waits for a response status and returns it if available
    pub async fn tell_and_get_status<T>(&mut self, req: Request<T>) -> Result<Option<Status>>
    where
        T: Encode<()>,
    {
        self.tell(req).await?;
        self.parse_response_status()
    }

    /// Send a request and expects a decodable response
    /// This method parses and returns an error message if the request was not successful
    pub async fn ask<T, R>(&mut self, req: Request<T>) -> Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.send_request(req).await?;
        self.parse_response_body::<R>()
    }

    /// Send a request and expects either a decodable response or an API error.
    /// This method returns an error if the request cannot be sent of if there is any decoding error
    pub async fn ask_and_get_reply<T, R>(&mut self, req: Request<T>) -> Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.send_request(req).await?;
        self.parse_response_reply::<R>()
    }

    /// Make a request and wait for a response
    /// This method _does not_ check the success of the request
    async fn send_request<T>(&mut self, req: Request<T>) -> Result<()>
    where
        T: Encode<()>,
    {
        let route = self.route_impl().await?;
        let options = self
            .timeout
            .map(|t| MessageSendReceiveOptions::new().with_timeout(t))
            .unwrap_or(MessageSendReceiveOptions::new());
        self.buf = self
            .ctx
            .send_and_receive_extended::<Vec<u8>>(route.clone(), req.to_vec()?, options)
            .await
            .map_err(|_err| {
                // Overwrite error to swallow inner cause and hide it from end-user
                miette!("The request timed out, please make sure the command's arguments are correct or try again")
            })?.body();
        Ok(())
    }

    async fn route_impl(&self) -> Result<Route> {
        let mut to = self.to.clone();
        let route = match &self.mode {
            RpcMode::Embedded => to,
            RpcMode::Background(tcp) => {
                let node_state = self.opts.state.nodes.get(&self.node_name)?;
                let port = node_state.config().setup().api_transport()?.addr.port();
                let addr_str = format!("localhost:{port}");
                let addr = tcp
                    .connect(addr_str, TcpConnectionOptions::new())
                    .await?
                    .sender_address()
                    .clone();
                to.modify().prepend(addr);
                to
            }
        };
        debug!(%route, "Sending request");
        Ok(route)
    }

    /// Parse the response body and return it
    /// This function returns an Err with a parsed error message if the response status is not ok
    fn parse_response_body<T>(&self) -> Result<T>
    where
        T: for<'b> Decode<'b, ()>,
    {
        Response::parse_response_body(self.buf.as_slice()).map_err(|e| miette!(e).into())
    }

    /// Parse the response body and return it
    /// This function returns an Err with a parsed error message if the response status is not ok
    fn parse_response_reply<T>(&self) -> Result<Reply<T>>
    where
        T: for<'b> Decode<'b, ()>,
    {
        Response::parse_response_reply(self.buf.as_slice()).map_err(|e| miette!(e).into())
    }

    /// Parse a Response and return its status
    fn parse_response_status(&self) -> Result<Option<Status>> {
        let (response, _decoder) = Response::parse_response_header(self.buf.as_slice())?;
        Ok(response.status())
    }
}

/// A simple wrapper for shutting down the local embedded node (for
/// the client side of the CLI).  Swallows errors and turns them into
/// eprintln logs.
///
/// TODO: We may want to change this behaviour in the future.
pub async fn stop_node(mut ctx: Context) {
    if let Err(e) = ctx.stop().await {
        eprintln!("an error occurred while shutting down local node: {e}");
    }
}

pub fn local_cmd(res: miette::Result<()>) {
    if let Err(e) = res {
        error!(%e, "Failed to run command");
        eprintln!("{:?}", e);
        std::process::exit(exitcode::SOFTWARE);
    }
}

pub fn node_rpc<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<()>> + Send + 'static,
{
    let res = embedded_node(
        |ctx, a| async {
            let res = f(ctx, a).await;
            if let Err(e) = res {
                error!(%e, "Failed to run command");
                eprintln!("{:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
            Ok(())
        },
        a,
    );
    if let Err(e) = res {
        eprintln!("Ockam runtime failed: {e}");
        std::process::exit(exitcode::SOFTWARE);
    }
}

pub fn embedded_node<A, F, Fut, T>(f: F, a: A) -> miette::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor
        .execute(async move {
            let child_ctx = ctx
                .new_detached(
                    Address::random_tagged("Detached.embedded_node"),
                    DenyAll,
                    DenyAll,
                )
                .await
                .expect("Embedded node child ctx can't be created");
            let r = f(child_ctx, a).await;
            stop_node(ctx).await;
            r
        })
        .into_diagnostic()?
}

pub fn embedded_node_that_is_not_stopped<A, F, Fut, T>(f: F, a: A) -> miette::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = miette::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (mut ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor
        .execute(async move {
            let child_ctx = ctx
                .new_detached(
                    Address::random_tagged("Detached.embedded_node.not_stopped"),
                    DenyAll,
                    DenyAll,
                )
                .await
                .expect("Embedded node child ctx can't be created");
            let result = f(child_ctx, a).await;
            if result.is_err() {
                ctx.stop().await.into_diagnostic()?;
                result
            } else {
                result
            }
        })
        .into_diagnostic()?
}

pub fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .into_diagnostic()
        .context("Unable to bind to an open port")?;
    let address = listener
        .local_addr()
        .into_diagnostic()
        .context("Unable to get local address")?;
    Ok(address.port())
}

#[allow(unused)]
pub fn print_path(p: &Path) -> String {
    p.to_str().unwrap_or("<unprintable>").to_string()
}

/// Parses a node's input string for its name in case it's a `MultiAddr` string.
///
/// Ensures that the node's name will be returned if the input string is a `MultiAddr` of the `node` type
/// Examples: `n1` or `/node/n1` returns `n1`; `/project/p1` or `/tcp/n2` returns an error message.
pub fn parse_node_name(input: &str) -> Result<String> {
    if input.is_empty() {
        return Err(miette!("Empty address in node name argument").into());
    }
    // Node name was passed as "n1", for example
    if !input.contains('/') {
        return Ok(input.to_string());
    }
    // Input has "/", so we process it as a MultiAddr
    let maddr = MultiAddr::from_str(input)
        .into_diagnostic()
        .wrap_err("Invalid format for node name argument")?;
    let err_message = String::from("A node MultiAddr must follow the format /node/<name>");
    if let Some(p) = maddr.iter().next() {
        if p.code() == proto::Node::CODE {
            let node_name = p
                .cast::<proto::Node>()
                .ok_or(miette!("Failed to parse the 'node' protocol"))?
                .to_string();
            if !node_name.is_empty() {
                return Ok(node_name);
            }
        }
    }
    Err(miette!(err_message).into())
}

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub fn process_nodes_multiaddr(addr: &MultiAddr, cli_state: &CliState) -> crate::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| miette!("Invalid node address protocol"))?;
                let node_state = cli_state.nodes.get(alias.to_string())?;
                let node_setup = node_state.config().setup();
                let addr = node_setup.api_transport()?.maddr()?;
                processed_addr.try_extend(&addr)?
            }
            _ => processed_addr.push_back_value(&proto)?,
        }
    }
    Ok(processed_addr)
}

/// Go through a multiaddr and remove all instances of
/// `/node/<whatever>` out of it and replaces it with a fully
/// qualified address to the target
pub fn clean_nodes_multiaddr(
    input: &MultiAddr,
    cli_state: &CliState,
) -> Result<(MultiAddr, LookupMeta)> {
    let mut new_ma = MultiAddr::default();
    let mut lookup_meta = LookupMeta::default();
    let it = input.iter().peekable();
    for p in it {
        match p.code() {
            Node::CODE => {
                let alias = p.cast::<Node>().expect("Failed to parse node name");
                let node_state = cli_state.nodes.get(alias.to_string())?;
                let node_setup = node_state.config().setup();
                let addr = &node_setup.api_transport()?.addr;
                match addr {
                    InternetAddress::Dns(dns, _) => new_ma.push_back(DnsAddr::new(dns))?,
                    InternetAddress::V4(v4) => new_ma.push_back(Ip4(*v4.ip()))?,
                    InternetAddress::V6(v6) => new_ma.push_back(Ip6(*v6.ip()))?,
                }
                new_ma.push_back(Tcp(addr.port()))?;
            }
            Project::CODE => {
                // Parse project name from the MultiAddr.
                let alias = p.cast::<Project>().expect("Failed to parse project name");
                // Store it in the lookup meta, so we can later
                // retrieve it from either the config or the cloud.
                lookup_meta.project.push_back(alias.to_string());
                // No substitution done here. It will be done later by `clean_projects_multiaddr`.
                new_ma.push_back_value(&p)?
            }
            Space::CODE => return Err(miette!("/space/ substitutions are not supported!").into()),
            _ => new_ma.push_back_value(&p)?,
        }
    }

    Ok((new_ma, lookup_meta))
}

pub fn comma_separated<T: AsRef<str>>(data: &[T]) -> String {
    use itertools::Itertools;

    #[allow(unstable_name_collisions)]
    data.iter().map(AsRef::as_ref).intersperse(", ").collect()
}

pub fn port_is_free_guard(address: &SocketAddr) -> Result<()> {
    let port = address.port();
    let ip = address.ip();
    if TcpListener::bind((ip, port)).is_err() {
        return Err(miette!("Another process is already listening on port {port}!").into());
    }
    Ok(())
}

pub fn is_tty<S: io_lifetimes::AsFilelike>(s: S) -> bool {
    use is_terminal::IsTerminal;
    s.is_terminal()
}

pub fn random_name() -> String {
    hex::encode(rand::random::<[u8; 4]>())
}

#[cfg(test)]
mod tests {
    use ockam_api::address::extract_address_value;
    use ockam_api::cli_state;
    use ockam_api::cli_state::identities::IdentityConfig;
    use ockam_api::cli_state::traits::StateDirTrait;
    use ockam_api::cli_state::{NodeConfig, VaultConfig};
    use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};

    use super::*;

    #[test]
    fn test_parse_node_name() {
        let test_cases = vec![
            ("", Err(())),
            ("test", Ok("test")),
            ("/test", Err(())),
            ("test/", Err(())),
            ("/node", Err(())),
            ("/node/", Err(())),
            ("/node/n1", Ok("n1")),
            ("/service/s1", Err(())),
            ("/project/p1", Err(())),
            ("/randomprotocol/rp1", Err(())),
            ("/node/n1/tcp", Err(())),
            ("/node/n1/test", Err(())),
            ("/node/n1/tcp/22", Ok("n1")),
        ];
        for (input, expected) in test_cases {
            if let Ok(addr) = expected {
                assert_eq!(parse_node_name(input).unwrap(), addr);
            } else {
                assert!(parse_node_name(input).is_err());
            }
        }
    }

    #[test]
    fn test_extract_address_value() {
        let test_cases = vec![
            ("", Err(())),
            ("test", Ok("test")),
            ("/test", Err(())),
            ("test/", Err(())),
            ("/node", Err(())),
            ("/node/", Err(())),
            ("/node/n1", Ok("n1")),
            ("/service/s1", Ok("s1")),
            ("/project/p1", Ok("p1")),
            ("/randomprotocol/rp1", Err(())),
            ("/node/n1/tcp", Err(())),
            ("/node/n1/test", Err(())),
            ("/node/n1/tcp/22", Ok("n1")),
        ];
        for (input, expected) in test_cases {
            if let Ok(addr) = expected {
                assert_eq!(extract_address_value(input).unwrap(), addr);
            } else {
                assert!(extract_address_value(input).is_err());
            }
        }
    }

    #[ockam_macros::test(crate = "ockam")]
    async fn test_process_multi_addr(ctx: &mut Context) -> ockam::Result<()> {
        let cli_state = CliState::test()?;

        let v_name = cli_state::random_name();
        let v_config = VaultConfig::default();
        cli_state.vaults.create_async(&v_name, v_config).await?;
        let v = cli_state.vaults.get(&v_name)?.get().await?;
        let idt = cli_state
            .get_identities(v)
            .await
            .unwrap()
            .identities_creation()
            .create_identity()
            .await?;
        let idt_config = IdentityConfig::new(idt.identifier()).await;
        cli_state
            .identities
            .create(cli_state::random_name(), idt_config)?;

        let n_state = cli_state
            .nodes
            .create("n1", NodeConfig::try_from(&cli_state)?)?;
        n_state.set_setup(&n_state.config().setup_mut().set_api_transport(
            CreateTransportJson::new(TransportType::Tcp, TransportMode::Listen, "127.0.0.0:4000")?,
        ))?;

        let test_cases = vec![
            (
                MultiAddr::from_str("/node/n1").unwrap(),
                Ok("/ip4/127.0.0.0/tcp/4000"),
            ),
            (
                MultiAddr::from_str("/project/p1").unwrap(),
                Ok("/project/p1"),
            ),
            (
                MultiAddr::from_str("/service/s1").unwrap(),
                Ok("/service/s1"),
            ),
            (
                MultiAddr::from_str("/project/p1/node/n1/service/echo").unwrap(),
                Ok("/project/p1/ip4/127.0.0.0/tcp/4000/service/echo"),
            ),
            (MultiAddr::from_str("/node/n2").unwrap(), Err(())),
        ];
        for (ma, expected) in test_cases {
            if let Ok(addr) = expected {
                let result = process_nodes_multiaddr(&ma, &cli_state)
                    .unwrap()
                    .to_string();
                assert_eq!(result, addr);
            } else {
                assert!(process_nodes_multiaddr(&ma, &cli_state).is_err());
            }
        }

        ctx.stop().await?;
        Ok(())
    }

    #[test]
    fn test_execute_error() {
        let result = embedded_node_that_is_not_stopped(function_returning_an_error, 1);
        assert!(result.is_err());

        async fn function_returning_an_error(_ctx: Context, _parameter: u8) -> miette::Result<()> {
            Err(miette!("boom"))
        }
    }

    #[test]
    fn test_execute_error_() {
        let result = embedded_node_that_is_not_stopped(
            function_returning_an_error_and_stopping_the_context,
            1,
        );
        assert!(result.is_err());

        async fn function_returning_an_error_and_stopping_the_context(
            mut ctx: Context,
            _parameter: u8,
        ) -> miette::Result<()> {
            ctx.stop().await.into_diagnostic()?;
            Err(miette!("boom"))
        }
    }
}
