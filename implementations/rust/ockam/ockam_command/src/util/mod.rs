use core::time::Duration;
use std::{
    env,
    net::{SocketAddr, TcpListener},
    path::Path,
    str::FromStr,
};

use anyhow::{anyhow, Context as _, Result};
use minicbor::{data::Type, Decode, Decoder, Encode};
use tracing::{debug, error, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub use config::*;
use ockam::{Address, Context, NodeBuilder, Route, TcpTransport, TCP};
use ockam_api::cli_state::{CliState, NodeState};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_core::DenyAll;
use ockam_multiaddr::{
    proto::{self, Node},
    MultiAddr, Protocol,
};

use crate::node::util::start_embedded_node;
use crate::util::output::Output;
use crate::{CommandGlobalOpts, OutputFormat};

pub mod api;
pub mod exitcode;

mod config;
pub(crate) mod output;

pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

#[derive(Clone)]
pub enum RpcMode<'a> {
    Embedded,
    Background {
        node_state: NodeState,
        tcp: Option<&'a TcpTransport>,
    },
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
        self.to = ockam_api::multiaddr_to_route(to)
            .ok_or_else(|| anyhow!("failed to convert {to} to route"))?;
        Ok(self)
    }

    /// When running multiple RPC's from a single command to a background node,
    /// a single TcpTransport must be shared amongst them, as we can only have one
    /// TcpTransport per Context.
    pub fn tcp<T: Into<Option<&'a TcpTransport>>>(mut self, tcp: T) -> Result<Self> {
        if let Some(tcp) = tcp.into() {
            self.mode = RpcMode::Background {
                node_state: self.opts.state.nodes.get(&self.node_name)?,
                tcp: Some(tcp),
            };
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
        let node_name = start_embedded_node(ctx, opts).await?;
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
        let cfg = opts.state.nodes.get(node_name)?;
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            mode: RpcMode::Background {
                node_state: cfg,
                tcp: None,
            },
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
            .context("Failed to receive response from node")?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn request_with_timeout<T>(
        &mut self,
        req: RequestBuilder<'_, T>,
        timeout: Duration,
    ) -> Result<()>
    where
        T: Encode<()>,
    {
        let route = self.route_impl(self.ctx).await?;
        self.buf = self
            .ctx
            .send_and_receive_with_timeout(route.clone(), req.to_vec()?, timeout)
            .await
            .context("Failed to receive response from node")?;
        Ok(())
    }

    async fn route_impl(&self, ctx: &Context) -> Result<Route> {
        let mut to = self.to.clone();
        let route = match self.mode {
            RpcMode::Embedded => to,
            RpcMode::Background {
                ref node_state,
                ref tcp,
            } => {
                let port = node_state.setup()?.default_tcp_listener()?.addr.port();
                let addr = Address::from((TCP, format!("localhost:{port}")));
                let addr_str = addr.address();
                match tcp {
                    None => {
                        let tcp = TcpTransport::create(ctx).await?;
                        tcp.connect(addr_str).await?;
                    }
                    Some(tcp) => {
                        // Ignore "already connected" error.
                        let _ = tcp.connect(addr_str).await;
                    }
                }
                to.modify().prepend_route(addr.into());
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
                Err(anyhow!("Failed to decode response body: {}", e))
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
            Err(anyhow!(msg))
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
        print_output(b, &self.opts.global_args.output_format)
    }
}

pub fn print_output<T>(b: T, output_format: &OutputFormat) -> Result<T>
where
    T: Output + serde::Serialize,
{
    let o = match output_format {
        OutputFormat::Plain => b.output().context("Failed to serialize output")?,
        OutputFormat::Json => {
            serde_json::to_string_pretty(&b).context("Failed to serialize output")?
        }
    };
    println!("{}", o);
    Ok(b)
}

/// A simple wrapper for shutting down the local embedded node (for
/// the client side of the CLI).  Swallows errors and turns them into
/// eprintln logs.
///
/// TODO: We may want to change this behaviour in the future.
pub async fn stop_node(mut ctx: Context) -> Result<()> {
    if let Err(e) = ctx.stop().await {
        eprintln!("an error occurred while shutting down local node: {}", e);
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
                eprintln!("{e:?}");
                std::process::exit(e.code());
            }
            Ok(())
        },
        a,
    );
    if let Err(e) = res {
        eprintln!("Ockam node failed: {e}");
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
    let (ctx, mut executor) = NodeBuilder::new().no_logging().build();
    executor.execute(async move {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Detached.embedded_node.not_stopped"),
                DenyAll,
                DenyAll,
            )
            .await
            .expect("Embedded node child ctx can't be created");
        f(child_ctx, a).await
    })?
}

pub fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("Unable to bind to an open port")?;
    let address = listener
        .local_addr()
        .context("Unable to get local address")?;
    Ok(address.port())
}

pub fn setup_logging(verbose: u8, no_color: bool) {
    let ockam_crates = [
        "ockam",
        "ockam_node",
        "ockam_core",
        "ockam_command",
        "ockam_identity",
        "ockam_transport_tcp",
        "ockam_vault",
        "ockam_vault_sync_core",
    ];
    let builder = EnvFilter::builder();
    // If `verbose` is not set, try to read the log level from the OCKAM_LOG env variable.
    // If both `verbose` and OCKAM_LOG are not set, logging will not be enabled.
    // Otherwise, use `verbose` to define the log level.
    let filter = match verbose {
        0 => match env::var("OCKAM_LOG") {
            Ok(s) if !s.is_empty() => builder.with_env_var("OCKAM_LOG").from_env_lossy(),
            _ => return,
        },
        1 => builder
            .with_default_directive(LevelFilter::INFO.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=info")).join(",")),
        2 => builder
            .with_default_directive(LevelFilter::DEBUG.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=debug")).join(",")),
        _ => builder
            .with_default_directive(LevelFilter::TRACE.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=trace")).join(",")),
    };
    let fmt = fmt::Layer::default().with_ansi(!no_color);
    let result = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .with(fmt)
        .try_init();
    if result.is_err() {
        eprintln!("Failed to initialise tracing logging.");
    }
}

#[allow(unused)]
pub fn print_path(p: &Path) -> String {
    p.to_str().unwrap_or("<unprintable>").to_string()
}

/// Get address value from a string.
///
/// The input string can be either a plain address of a MultiAddr formatted string.
/// Examples: `/node/<name>`, `<name>`
pub fn extract_address_value(input: &str) -> anyhow::Result<String> {
    // we default to the `input` value
    let mut addr = input.to_string();
    // if input has "/", we process it as a MultiAddr
    if input.contains('/') {
        let err = anyhow!("invalid address protocol");
        let maddr = MultiAddr::from_str(input)?;
        if let Some(p) = maddr.iter().next() {
            match p.code() {
                proto::Node::CODE => {
                    addr = p
                        .cast::<proto::Node>()
                        .context("Failed to parse `node` protocol")?
                        .to_string();
                }
                proto::Service::CODE => {
                    addr = p
                        .cast::<proto::Service>()
                        .context("Failed to parse `service` protocol")?
                        .to_string();
                }
                proto::Project::CODE => {
                    addr = p
                        .cast::<proto::Project>()
                        .context("Failed to parse `project` protocol")?
                        .to_string();
                }
                code => return Err(anyhow!("Protocol {code} not supported")),
            }
        } else {
            return Err(err);
        }
    }
    if addr.is_empty() {
        return Err(anyhow!("Empty address in input: {input}"));
    }
    Ok(addr)
}

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub fn process_multi_addr(addr: &MultiAddr, cli_state: &CliState) -> anyhow::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| anyhow!("invalid node address protocol"))?;
                let node_setup = cli_state.nodes.get(&alias)?.setup()?;
                let addr = node_setup.default_tcp_listener()?.maddr()?;
                processed_addr.try_extend(&addr)?
            }
            _ => processed_addr.push_back_value(&proto)?,
        }
    }
    Ok(processed_addr)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::cli_state;
    use ockam_api::cli_state::{IdentityConfig, NodeConfig, VaultConfig};
    use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
    use ockam_identity::Identity;
    use ockam_vault::storage::FileStorage;
    use ockam_vault::Vault;
    use std::sync::Arc;

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
        let v_config = VaultConfig::from_name(&v_name)?;
        let v_storage = FileStorage::create(VaultConfig::path(&v_name)?).await?;
        let v = Vault::new(Some(Arc::new(v_storage)));
        cli_state.vaults.create(&v_name, v_config).await?;

        let idt = Identity::create(ctx, &v).await?;
        let idt_config = IdentityConfig::new(&idt).await;
        cli_state
            .identities
            .create(&cli_state::random_name(), idt_config)?;

        let n_state = cli_state.nodes.create("n1", NodeConfig::try_default()?)?;
        let n_setup = n_state.setup()?;
        n_state.set_setup(&n_setup.add_transport(CreateTransportJson::new(
            TransportType::Tcp,
            TransportMode::Listen,
            "127.0.0.0:4000",
        )?))?;

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
                let result = process_multi_addr(&ma, &cli_state).unwrap().to_string();
                assert_eq!(result, addr);
            } else {
                assert!(process_multi_addr(&ma, &cli_state).is_err());
            }
        }

        ctx.stop().await?;
        Ok(())
    }
}
