use core::time::Duration;
use std::{
    env,
    net::{SocketAddr, TcpListener},
    path::Path,
};

use anyhow::{anyhow, Context as _, Result};
use crossbeam_channel::{bounded, Sender};
use minicbor::{Decode, Decoder, Encode};
use tracing::{debug, error, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub use addon::AddonCommand;
pub use config::*;
use ockam::{route, Address, Context, NodeBuilder, Route, TcpTransport, TCP};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_multiaddr::MultiAddr;

use crate::node::util::start_embedded_node;
use crate::util::output::Output;
use crate::{CommandGlobalOpts, OutputFormat};

pub mod api;
pub mod exitcode;
pub mod startup;

mod addon;
mod config;
pub(crate) mod output;

pub const DEFAULT_ORCHESTRATOR_ADDRESS: &str =
    "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

pub enum RpcMode<'a> {
    Embedded,
    Background {
        cfg: NodeConfig,
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
            let cfg = self.opts.config.get_node(&self.node_name)?;
            self.mode = RpcMode::Background {
                cfg,
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
        let node_name = start_embedded_node(ctx, &opts.config).await?;
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
        let cfg = opts.config.get_node(node_name)?;
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            mode: RpcMode::Background { cfg, tcp: None },
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
        let mut ctx = self.ctx.new_detached(Address::random_local()).await?;
        let route = self.route_impl(&ctx).await?;
        ctx.send(route.clone(), req.to_vec()?).await?;
        self.buf = ctx
            .receive_duration_timeout::<Vec<u8>>(timeout)
            .await
            .context("Failed to receive response from node")?
            .take()
            .body();
        Ok(())
    }

    async fn route_impl(&mut self, ctx: &Context) -> Result<Route> {
        let route = match self.mode {
            RpcMode::Embedded => self.to.clone(),
            RpcMode::Background { ref cfg, ref tcp } => {
                let addr = Address::from((TCP, format!("localhost:{}", cfg.port)));
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
                self.to.modify().prepend_route(addr.into()).into()
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
        dec.decode().context("Failed to decode response body")
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
                let err = match dec.decode::<String>() {
                    Ok(msg) => format!("Message: {msg}"),
                    Err(_) => String::default(),
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
        let o = match self.opts.global_args.output_format {
            OutputFormat::Plain => b.output().context("Failed to serialize response body")?,
            OutputFormat::Json => {
                serde_json::to_string_pretty(&b).context("Failed to serialize response body")?
            }
        };
        println!("{}", o);
        Ok(b)
    }
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

/// Connect to a remote node (on localhost for now)
///
/// This function requires the "remote" port, some command payload,
/// and a user function to run.  It uses `embedded_node` internally,
/// while also configuring a TcpTransport and connecting to another
/// node.
///
pub fn connect_to<A, F, Fut>(port: u16, a: A, lambda: F)
where
    A: Send + Sync + 'static,
    F: FnOnce(Context, A, Route) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = Result<()>> + Send + 'static,
{
    let res = embedded_node(
        move |ctx, a| async move {
            let tcp = match TcpTransport::create(&ctx).await {
                Ok(tcp) => tcp,
                Err(e) => {
                    eprintln!("failed to create TcpTransport");
                    error!(%e);
                    std::process::exit(exitcode::CANTCREAT);
                }
            };
            if let Err(e) = tcp.connect(format!("localhost:{}", port)).await {
                eprintln!("failed to connect to node");
                error!(%e);
                std::process::exit(exitcode::IOERR);
            }
            let route = route![(TCP, format!("localhost:{}", port))];
            if let Err(e) = lambda(ctx, a, route).await {
                eprintln!("encountered an error in command handler code");
                error!(%e);
                std::process::exit(exitcode::IOERR);
            }
            Ok(())
        },
        a,
    );
    if let Err(e) = res {
        eprintln!("Ockam node failed: {:?}", e,);
    }
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
                error!("{e:?}");
                eprintln!("{e}");
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
    let (ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();
    let r = executor.execute(async move {
        let child_ctx = ctx
            .new_detached(Address::random_local())
            .await
            .expect("Embedded node child ctx can't be created");
        let r = f(child_ctx, a).await;
        stop_node(ctx).await.unwrap();
        match r {
            Err(e) => {
                error!("{e:?}");
                eprintln!("{e}");
                std::process::exit(e.code());
            }
            Ok(v) => v,
        }
    })?;
    Ok(r)
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
        "ockam_channel",
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

pub fn get_final_element(input_path: &str) -> &str {
    //  Get Node name from Node Path
    //  if Input path has "/", we split the path and return the final element
    //  if the final element is empty string, we return None
    return if input_path.contains('/') {
        let split_path: Vec<&str> = input_path.split('/').collect();
        match split_path.last() {
            Some(last_value) if last_value.is_empty() => {
                eprintln!("Invalid Format: {}", input_path);
                std::process::exit(exitcode::IOERR);
            }
            Some(last_value) => last_value,
            None => input_path,
        }
    } else {
        input_path
    };
}

pub fn comma_separated<T: AsRef<str>>(data: &[T]) -> String {
    use itertools::Itertools;

    #[allow(unstable_name_collisions)]
    data.iter().map(AsRef::as_ref).intersperse(", ").collect()
}

pub fn bind_to_port_check(address: &SocketAddr) -> bool {
    let port = address.port();
    let ip = address.ip();
    std::net::TcpListener::bind((ip, port)).is_ok()
}

pub fn verify_pids(cfg: &OckamConfig, nodes: Vec<String>) {
    for node_name in nodes {
        let node_cfg = cfg.get_node(&node_name).unwrap();

        let (tx, rx) = bounded(1);

        connect_to(node_cfg.port, tx, query_pid);
        let verified_pid = rx.recv().unwrap();

        if node_cfg.pid != verified_pid {
            if let Err(e) = cfg.set_node_pid(&node_name, verified_pid) {
                eprintln!("failed to update pid for node {}: {}", node_name, e);
                std::process::exit(exitcode::IOERR);
            }
        }
    }

    if cfg.persist_config_updates().is_err() {
        eprintln!("failed to update PID information in config!");
        std::process::exit(exitcode::IOERR);
    }
}

pub async fn query_pid(
    mut ctx: Context,
    tx: Sender<Option<i32>>,
    mut base_route: Route,
) -> anyhow::Result<()> {
    ctx.send(
        base_route.modify().append(NODEMANAGER_ADDR),
        api::query_status()?,
    )
    .await?;

    let resp = match ctx
        .receive_duration_timeout::<Vec<u8>>(Duration::from_millis(200))
        .await
    {
        Ok(r) => r.take().body(),
        Err(_) => {
            tx.send(None).unwrap();
            return Ok(());
        }
    };

    let status = api::parse_status(&resp)?;
    tx.send(Some(status.pid)).unwrap();
    Ok(())
}
