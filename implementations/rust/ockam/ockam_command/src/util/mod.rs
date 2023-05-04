use core::time::Duration;

use std::{
    net::{SocketAddr, TcpListener},
    path::Path,
    str::FromStr,
};

use anyhow::{anyhow, Context as _};
use minicbor::{data::Type, Decode, Decoder, Encode};
use tracing::{debug, error, trace};

pub use config::*;
use ockam::{
    Address, Context, MessageSendReceiveOptions, NodeBuilder, Route, TcpConnectionOptions,
    TcpTransport,
};
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::config::lookup::{InternetAddress, LookupMeta};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_core::DenyAll;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Service, Space, Tcp};
use ockam_multiaddr::{
    proto::{self, Node},
    MultiAddr, Protocol,
};

use crate::util::output::Output;
use crate::{node::util::start_embedded_node, EncodeFormat};
use crate::{CommandGlobalOpts, OutputFormat, Result};

pub mod api;
pub mod exitcode;
pub mod orchestrator_api;
pub mod parsers;

mod config;
pub(crate) mod output;

pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

#[derive(Clone)]
pub enum RpcMode<'a> {
    Embedded,
    Background { tcp: Option<&'a TcpTransport> },
}

pub struct RpcBuilder<'a> {
    ctx: &'a Context,
    opts: &'a CommandGlobalOpts,
    node_name: String,
    to: Route,
    mode: RpcMode<'a>,
}

impl<'a> RpcBuilder<'a> {
    pub fn new(ctx: &'a Context, opts: &'a CommandGlobalOpts, node_name: &str) -> Self {
        RpcBuilder {
            ctx,
            opts,
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            mode: RpcMode::Embedded,
        }
    }

    pub fn to(mut self, to: &MultiAddr) -> Result<Self> {
        self.to = ockam_api::local_multiaddr_to_route(to)
            .ok_or_else(|| anyhow!("failed to convert {to} to route"))?;
        Ok(self)
    }

    /// When running multiple RPC's from a single command to a background node,
    /// a single TcpTransport must be shared amongst them, as we can only have one
    /// TcpTransport per Context.
    pub fn tcp<T: Into<Option<&'a TcpTransport>>>(mut self, tcp: T) -> Result<Self> {
        if let Some(tcp) = tcp.into() {
            self.mode = RpcMode::Background { tcp: Some(tcp) };
        }
        Ok(self)
    }

    pub fn build(self) -> Rpc<'a> {
        Rpc {
            ctx: self.ctx,
            buf: Vec::new(),
            opts: self.opts,
            node_name: self.node_name,
            to: self.to,
            mode: self.mode,
        }
    }
}

#[derive(Clone)]
pub struct Rpc<'a> {
    ctx: &'a Context,
    buf: Vec<u8>,
    opts: &'a CommandGlobalOpts,
    node_name: String,
    to: Route,
    mode: RpcMode<'a>,
}

impl<'a> Rpc<'a> {
    /// Creates a new RPC to send a request to an embedded node.
    pub async fn embedded(ctx: &'a Context, opts: &'a CommandGlobalOpts) -> Result<Rpc<'a>> {
        let node_name = start_embedded_node(ctx, opts, None).await?;
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            node_name,
            to: NODEMANAGER_ADDR.into(),
            mode: RpcMode::Embedded,
        })
    }

    /// Creates a new RPC to send a request to a running background node.
    pub fn background(
        ctx: &'a Context,
        opts: &'a CommandGlobalOpts,
        node_name: &str,
    ) -> Result<Rpc<'a>> {
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            mode: RpcMode::Background { tcp: None },
        })
    }

    pub fn node_name(&self) -> &str {
        &self.node_name
    }

    pub async fn request<T>(&mut self, req: RequestBuilder<'_, T>) -> Result<()>
    where
        T: Encode<()>,
    {
        let route = self.route_impl(self.ctx).await?;
        self.buf = self
            .ctx
            .send_and_receive(route.clone(), req.to_vec()?)
            .await
            .map_err(|_err| {
                // Overwrite error to swallow inner cause and hide it from end-user
                anyhow!("The request timed out, please make sure the command's arguments are correct or try again")
            })?;
        Ok(())
    }

    pub async fn request_with_timeout<T>(
        &mut self,
        req: RequestBuilder<'_, T>,
        timeout: Duration,
    ) -> Result<()>
    where
        T: Encode<()>,
    {
        let route = self.route_impl(self.ctx).await?;
        let options = MessageSendReceiveOptions::new().with_timeout(timeout);
        self.buf = self
            .ctx
            .send_and_receive_extended::<Vec<u8>>(route.clone(), req.to_vec()?, options)
            .await
            .map_err(|_err| {
                // Overwrite error to swallow inner cause and hide it from end-user
                anyhow!("The request timed out, please make sure the command's arguments are correct or try again")
            })?.body();
        Ok(())
    }

    async fn route_impl(&self, ctx: &Context) -> Result<Route> {
        let mut to = self.to.clone();
        let route = match self.mode {
            RpcMode::Embedded => to,
            RpcMode::Background { ref tcp } => {
                let node_state = self.opts.state.nodes.get(&self.node_name)?;
                let port = node_state
                    .config()
                    .setup()
                    .default_tcp_listener()?
                    .addr
                    .port();
                let addr_str = format!("localhost:{port}");
                let addr = match tcp {
                    None => {
                        let tcp = TcpTransport::create(ctx).await?;
                        tcp.connect(addr_str, TcpConnectionOptions::new())
                            .await?
                            .sender_address()
                            .clone()
                    }
                    Some(tcp) => {
                        // Create a new connection anyway
                        tcp.connect(addr_str, TcpConnectionOptions::new())
                            .await?
                            .sender_address()
                            .clone()
                    }
                };
                to.modify().prepend(addr);
                to
            }
        };
        debug!(%route, "Sending request");
        Ok(route)
    }

    /// Parse the response body and return it.
    pub fn parse_response<T>(&'a self) -> Result<T>
    where
        T: Decode<'a, ()>,
    {
        let mut dec = self.parse_response_impl()?;
        match dec.decode() {
            Ok(t) => Ok(t),
            Err(e) => {
                error!(%e, dec = %minicbor::display(&self.buf), hex = %hex::encode(&self.buf), "Failed to decode response");
                Err(anyhow!("Failed to decode response body: {}", e).into())
            }
        }
    }

    /// Check response's status code is OK.
    pub fn is_ok(&self) -> Result<()> {
        self.parse_response_impl()?;
        Ok(())
    }

    pub fn check_response(&self) -> Result<(Response, Decoder)> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        Ok((hdr, dec))
    }

    /// Parse the header and returns the decoder.
    fn parse_response_impl(&self) -> Result<Decoder> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        if hdr.status() == Some(Status::Ok) {
            Ok(dec)
        } else {
            let msg = self.parse_err_msg(hdr, dec);
            Err(anyhow!(msg).into())
        }
    }

    pub fn parse_err_msg(&self, hdr: Response, mut dec: Decoder) -> String {
        trace! {
            dec = %minicbor::display(&self.buf),
            hex = %hex::encode(&self.buf),
            "Received CBOR message"
        };
        match hdr.status() {
            Some(status) if hdr.has_body() => {
                let err = if matches!(dec.datatype(), Ok(Type::String)) {
                    dec.decode::<String>()
                        .map(|msg| format!("Message: {msg}"))
                        .unwrap_or_default()
                } else {
                    dec.decode::<ockam_core::api::Error>()
                        .map(|e| {
                            e.message()
                                .map(|msg| format!("Message: {msg}"))
                                .unwrap_or_default()
                        })
                        .unwrap_or_default()
                };
                format!(
                    "An error occurred while processing the request. Status code: {status}. {err}"
                )
            }
            Some(status) => {
                format!("An error occurred while processing the request. Status code: {status}")
            }
            None => "No status code found in response".to_string(),
        }
    }

    /// Parse the response body and print it.
    pub fn parse_and_print_response<T>(&'a self) -> Result<T>
    where
        T: Decode<'a, ()> + Output + serde::Serialize,
    {
        let b: T = self.parse_response()?;
        self.print_response(b)
    }

    pub fn print_response<T>(&self, b: T) -> Result<T>
    where
        T: Output + serde::Serialize,
    {
        println_output(b, &self.opts.global_args.output_format)
    }
}

pub fn println_output<T>(b: T, output_format: &OutputFormat) -> Result<T>
where
    T: Output + serde::Serialize,
{
    let o = get_output(&b, output_format)?;
    println!("{o}");
    Ok(b)
}

fn get_output<T>(b: &T, output_format: &OutputFormat) -> Result<String>
where
    T: Output + serde::Serialize,
{
    let output = match output_format {
        OutputFormat::Plain => b.output().context("Failed to serialize output")?,
        OutputFormat::Json => {
            serde_json::to_string_pretty(b).context("Failed to serialize output")?
        }
    };
    Ok(output)
}

pub fn print_encodable<T>(e: T, encode_format: &EncodeFormat) -> Result<()>
where
    T: Encode<()> + Output,
{
    let o = match encode_format {
        EncodeFormat::Plain => e.output().context("Failed serialize output")?,
        EncodeFormat::Hex => {
            let bytes = minicbor::to_vec(e).expect("Unable to encode response");
            hex::encode(bytes)
        }
    };

    print!("{o}");
    Ok(())
}

/// A simple wrapper for shutting down the local embedded node (for
/// the client side of the CLI).  Swallows errors and turns them into
/// eprintln logs.
///
/// TODO: We may want to change this behaviour in the future.
pub async fn stop_node(mut ctx: Context) -> Result<()> {
    if let Err(e) = ctx.stop().await {
        eprintln!("an error occurred while shutting down local node: {e}");
    }
    Ok(())
}

pub fn node_rpc<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = crate::Result<()>> + Send + 'static,
{
    let res = embedded_node(
        |ctx, a| async {
            let res = f(ctx, a).await;
            if let Err(e) = res {
                error!(%e);
                eprint!("{e}");
                std::process::exit(e.code());
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

pub fn embedded_node<A, F, Fut, T>(f: F, a: A) -> crate::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = crate::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor.execute(async move {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Detached.embedded_node"),
                DenyAll,
                DenyAll,
            )
            .await
            .expect("Embedded node child ctx can't be created");
        let r = f(child_ctx, a).await;
        stop_node(ctx).await.unwrap();
        r
    })?
}

pub fn embedded_node_that_is_not_stopped<A, F, Fut, T>(f: F, a: A) -> crate::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = crate::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (mut ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor.execute(async move {
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
            ctx.stop().await?;
            result
        } else {
            result
        }
    })?
}

pub fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("Unable to bind to an open port")?;
    let address = listener
        .local_addr()
        .context("Unable to get local address")?;
    Ok(address.port())
}

#[allow(unused)]
pub fn print_path(p: &Path) -> String {
    p.to_str().unwrap_or("<unprintable>").to_string()
}

/// Get address value from a string.
///
/// The input string can be either a plain address of a MultiAddr formatted string.
/// Examples: `/node/<name>`, `<name>`
pub fn extract_address_value(input: &str) -> Result<String> {
    // we default to the `input` value
    let mut addr = input.to_string();
    // if input has "/", we process it as a MultiAddr
    if input.contains('/') {
        let err = anyhow!("invalid address protocol");
        let maddr = MultiAddr::from_str(input)?;
        if let Some(p) = maddr.iter().next() {
            match p.code() {
                Node::CODE => {
                    addr = p
                        .cast::<proto::Node>()
                        .context("Failed to parse `node` protocol")?
                        .to_string();
                }
                Service::CODE => {
                    addr = p
                        .cast::<proto::Service>()
                        .context("Failed to parse `service` protocol")?
                        .to_string();
                }
                Project::CODE => {
                    addr = p
                        .cast::<proto::Project>()
                        .context("Failed to parse `project` protocol")?
                        .to_string();
                }
                code => return Err(anyhow!("Protocol {code} not supported").into()),
            }
        } else {
            return Err(err.into());
        }
    }
    if addr.is_empty() {
        return Err(anyhow!("Empty address in input: {input}").into());
    }
    Ok(addr)
}

/// Parses a node's input string for its name in case it's a `MultiAddr` string. Wraps around `extract_address_value`
///
/// Ensures that the node's name will be returned if the input string is a `MultiAddr` of the `node` type
/// Examples:
///     `node create n1` returns n1, `node create /node/n1` returns n1, `node create /tcp/n2` returns an error message.
pub fn parse_node_name(input: &str) -> Result<String> {
    let addr = input.to_string();
    // if input has "/", we process it as a MultiAddr like in `extract_address_value`
    if input.contains('/') {
        let mut err_message =
            String::from("A MultiAddr node must follow the format /node/{{name}}");
        let maddr = match MultiAddr::from_str(input) {
            Ok(maddr) => maddr,
            Err(e) => {
                // Tested with input strings with large tcp numbers. e.g: `node create /node/n1/tcp/28273829`
                err_message.push_str(&format!("\n{e}"));
                return Err(anyhow!(err_message).into());
            }
        };
        if let Some(p) = maddr.iter().next() {
            if p.code() == proto::Node::CODE {
                let node_name = extract_address_value(&addr)?;
                return Ok(node_name);
            }
        } else {
            return Err(anyhow!(err_message).into());
        }
    }
    if addr.is_empty() {
        return Err(anyhow!("Empty address in node name input: {input}").into());
    }
    Ok(addr)
}

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub fn process_nodes_multiaddr(
    addr: &MultiAddr,
    cli_state: &CliState,
) -> anyhow::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| anyhow!("invalid node address protocol"))?;
                let node_state = cli_state.nodes.get(&alias)?;
                let node_setup = node_state.config().setup();
                let addr = node_setup.default_tcp_listener()?.maddr()?;
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
                let node_state = cli_state.nodes.get(&alias)?;
                let node_setup = node_state.config().setup();
                let addr = &node_setup.default_tcp_listener()?.addr;
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
            Space::CODE => panic!("/space/ substitutions are not supported yet!"),
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

pub fn bind_to_port_check(address: &SocketAddr) -> bool {
    let port = address.port();
    let ip = address.ip();
    TcpListener::bind((ip, port)).is_ok()
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
    use super::*;
    use ockam_api::cli_state;
    use ockam_api::cli_state::identities::IdentityConfig;
    use ockam_api::cli_state::traits::StateDirTrait;
    use ockam_api::cli_state::{NodeConfig, VaultConfig};
    use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};

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
        let idt_config = IdentityConfig::new(&idt).await;
        cli_state
            .identities
            .create(&cli_state::random_name(), idt_config)?;

        let n_state = cli_state
            .nodes
            .create("n1", NodeConfig::try_from(&cli_state)?)?;
        n_state.set_setup(&n_state.config().setup_mut().add_transport(
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

        async fn function_returning_an_error(_ctx: Context, _parameter: u8) -> crate::Result<()> {
            Err(crate::Error::new(exitcode::CONFIG, anyhow!("boom")))
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
        ) -> crate::Result<()> {
            ctx.stop().await?;
            Err(crate::Error::new(exitcode::CONFIG, anyhow!("boom")))
        }
    }
}
