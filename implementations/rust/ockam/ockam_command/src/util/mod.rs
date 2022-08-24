use core::time::Duration;
use std::{
    env,
    net::{SocketAddr, TcpListener},
    path::Path, marker::PhantomData,
};

use anyhow::{anyhow, Context};
use minicbor::{Decode, Decoder, Encode};
use tracing::{debug, error, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub use addon::AddonCommand;
pub use config::*;
use ockam::{route, Address, NodeBuilder, Route, TcpTransport, TCP};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_multiaddr::MultiAddr;

use crate::util::output::Output;
use crate::{CommandGlobalOpts, OutputFormat};

pub mod api;
pub mod exitcode;
pub mod startup;

mod addon;
mod config;
pub(crate) mod output;

pub const DEFAULT_ORCHESTRATOR_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/62526";

pub struct RpcBuilder<'a, 'b> {
    ctx: &'a ockam::Context,
    opts: &'a CommandGlobalOpts,
    node: &'b str,
    to: Route,
    tcp: Option<&'a TcpTransport>,
}

impl<'a, 'b> RpcBuilder<'a, 'b> {
    pub fn new(ctx: &'a ockam::Context, opts: &'a CommandGlobalOpts, node: &'b str) -> Self {
        RpcBuilder {
            ctx,
            opts,
            node,
            to: NODEMANAGER_ADDR.into(),
            tcp: None,
        }
    }

    pub fn to(mut self, to: &MultiAddr) -> anyhow::Result<Self> {
        self.to = ockam_api::multiaddr_to_route(to)
            .ok_or_else(|| anyhow!("failed to convert {to} to route"))?;
        Ok(self)
    }

    pub fn tcp(mut self, tcp: &'a TcpTransport) -> Self {
        self.tcp = Some(tcp);
        self
    }

    pub fn build(self) -> anyhow::Result<Rpc<'a>> {
        let mut rpc = Rpc::new(self.ctx, self.opts, self.node)?;
        rpc.to = self.to;
        rpc.tcp = self.tcp;
        Ok(rpc)
    }
}

pub struct Rpc<'a> {
    ctx: &'a ockam::Context,
    buf: Vec<u8>,
    opts: &'a CommandGlobalOpts,
    cfg: NodeConfig,
    to: Route,
    /// Needed for when we want to call multiple Rpc's from a single command.
    tcp: Option<&'a TcpTransport>,
}

impl<'a> Rpc<'a> {
    pub fn new(
        ctx: &'a ockam::Context,
        opts: &'a CommandGlobalOpts,
        node: &str,
    ) -> anyhow::Result<Rpc<'a>> {
        let cfg = opts.config.get_node(node)?;
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            cfg,
            to: NODEMANAGER_ADDR.into(),
            tcp: None,
        })
    }

    pub async fn request<T>(&mut self, req: RequestBuilder<'_, T>) -> anyhow::Result<()>
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

    pub async fn request_with_timeout<T>(
        &mut self,
        req: RequestBuilder<'_, T>,
        timeout: Duration,
    ) -> anyhow::Result<()>
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

    async fn route_impl(&mut self, ctx: &ockam::Context) -> anyhow::Result<Route> {
        let addr = node_addr(&self.cfg);
        let addr_str = addr.address();

        match self.tcp {
            None => {
                let tcp = TcpTransport::create(ctx).await?;
                tcp.connect(addr_str).await?;
            }
            Some(tcp) => {
                // Ignore "already connected" error.
                let _ = tcp.connect(addr_str).await;
            }
        }

        let route = self.to.modify().prepend_route(addr.into()).into();
        debug!(%route, "Sending request");
        Ok(route)
    }

    /// Parse the response body and return it.
    pub fn parse_response<T>(&'a self) -> crate::Result<T>
    where
        T: Decode<'a, ()>,
    {
        let mut dec = self.parse_response_impl()?;
        Ok(dec.decode().context("Failed to decode response body")?)
    }

    /// Check response's status code is OK.
    pub fn is_ok(&self) -> crate::Result<()> {
        self.parse_response_impl()?;
        Ok(())
    }

    pub fn check_response(&self) -> crate::Result<(Response, Decoder)> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        Ok((hdr, dec))
    }

    /// Parse the header and returns the decoder.
    fn parse_response_impl(&self) -> crate::Result<Decoder> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        if hdr.status() == Some(Status::Ok) {
            Ok(dec)
        } else {
            eprintln!("{}", self.parse_err_msg(hdr, dec));
            Err(crate::Error::new(exitcode::SOFTWARE))
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
                    Ok(msg) => msg,
                    Err(_) => dec.decode::<String>().unwrap_or_default(),
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
    pub fn print_response<T>(&'a self) -> crate::Result<()>
    where
        T: Decode<'a, ()> + Output + serde::Serialize,
    {
        let b: T = self.parse_response()?;
        let o = match self.opts.global_args.output_format {
            OutputFormat::Plain => b.output().context("Failed to serialize response body")?,
            OutputFormat::Json => {
                serde_json::to_string_pretty(&b).context("Failed to serialize response body")?
            }
        };
        println!("{}", o);
        Ok(())
    }
}

pub trait CmdTrait<'a>: 'a {
    type Req: Encode<()>;
    type Resp: Decode<'a, ()> + Output + serde::Serialize;

    fn req(&'a mut self) -> RequestBuilder<'a, Self::Req>;
    fn parse_response(&'a self, res: &'a Vec<u8>) -> crate::Result<Self::Resp> {
        let mut dec = Decoder::new(&res);
        let res: Self::Resp = dec.decode().context("Failed to decode response body")?; 
        Ok(res)   
    }
}
pub struct RpcBuilder1<'c: 'd, 'd, 'b, T: CmdTrait<'d>> {
    ctx: &'c ockam::Context,
    cmd: &'c mut T,
    opts: &'c CommandGlobalOpts,
    node: &'b str,
    to: Route,
    tcp: Option<&'c TcpTransport>,
    _marker: &'d PhantomData<()>,
}

impl<'c, 'd, 'b, T: CmdTrait<'d>> RpcBuilder1<'c, 'd, 'b, T> {
    pub fn new(ctx: &'c ockam::Context, cmd: &'c mut T, opts: &'c CommandGlobalOpts, node: &'b str) -> Self {
        Self {
            ctx,
            cmd,
            opts,
            node,
            to: NODEMANAGER_ADDR.into(),
            tcp: None,
            _marker: &PhantomData,
        }
    }

    pub fn to(mut self, to: &MultiAddr) -> anyhow::Result<Self> {
        self.to = ockam_api::multiaddr_to_route(to)
            .ok_or_else(|| anyhow!("failed to convert {to} to route"))?;
        Ok(self)
    }

    pub fn tcp(mut self, tcp: &'c TcpTransport) -> Self {
        self.tcp = Some(tcp);
        self
    }

    pub fn build(self) -> anyhow::Result<Rpc1<'c, 'd, T>> {
        let mut rpc = Rpc1::new(self.ctx, self.cmd, self.opts, self.node)?;
        rpc.to = self.to;
        rpc.tcp = self.tcp;
        Ok(rpc)
    }
}
pub struct Rpc1<'c: 'd, 'd, T: CmdTrait<'d>> {
    ctx: &'c ockam::Context,
    cmd: &'c mut T,
    buf: Vec<u8>,
    opts: &'c CommandGlobalOpts,
    cfg: NodeConfig,
    to: Route,
    /// Needed for when we want to call multiple Rpc's from a single command.
    tcp: Option<&'c TcpTransport>,
    _marker: &'d PhantomData<T>,
}

impl<'c: 'd, 'd, T: CmdTrait<'d>> Rpc1<'c, 'd, T> {
    pub fn new(
        ctx: &'c ockam::Context,
        cmd: &'c mut T,
        opts: &'c CommandGlobalOpts,
        node: &str,
    ) -> anyhow::Result<Self> {
        let cfg = opts.config.get_node(node)?;
        Ok(Rpc1 {
            ctx,
            cmd,
            buf: Vec::new(),
            opts,
            cfg,
            to: NODEMANAGER_ADDR.into(),
            tcp: None,
            _marker: &PhantomData,
        })
    }

    pub async fn request_then_response(&'d mut self) -> crate::Result<Vec<u8>>
    //-> crate::Result<<T as CmdTrait<'d>>::Resp>
    {
        let route = self.route_impl(self.ctx).await.map_err(anyhow::Error::from)?;
//        self.buf = self
        let buf = self
            .ctx
            .send_and_receive(route.clone(), self.cmd.req().to_vec().map_err(anyhow::Error::from)?)
            .await
            .context("Failed to receive response from node")?;
/*        let mut dec = self.parse_response_impl()?;
        Ok(dec.decode().context("Failed to decode response body")?)
*/
        Ok(buf)
//        Ok(())
    }

    pub async fn request_with_timeout(
        &'d mut self,
        timeout: Duration,
    ) -> anyhow::Result<()>
    {
        let mut ctx = self.ctx.new_detached(Address::random_local()).await?;
        let route = self.route_impl(&ctx).await?;
        ctx.send(route.clone(), self.cmd.req().to_vec()?).await?;
        self.buf = ctx
            .receive_duration_timeout::<Vec<u8>>(timeout)
            .await
            .context("Failed to receive response from node")?
            .take()
            .body();
        Ok(())
    }

    async fn route_impl(&mut self, ctx: &ockam::Context) -> anyhow::Result<Route> {
        let addr = node_addr(&self.cfg);
        let addr_str = addr.address();

        match self.tcp {
            None => {
                let tcp = TcpTransport::create(ctx).await?;
                tcp.connect(addr_str).await?;
            }
            Some(tcp) => {
                // Ignore "already connected" error.
                let _ = tcp.connect(addr_str).await;
            }
        }

        let route = self.to.modify().prepend_route(addr.into()).into();
        debug!(%route, "Sending request");
        Ok(route)
    }

    /// Parse the response body and return it.
    pub fn parse_response(&'d self) -> crate::Result<T::Resp>
    {
        let mut dec = self.parse_response_impl()?;
        Ok(dec.decode().context("Failed to decode response body")?)
    }

    /// Check response's status code is OK.
    pub fn is_ok(&self) -> crate::Result<()> {
        self.parse_response_impl()?;
        Ok(())
    }

    pub fn check_response(&self) -> crate::Result<(Response, Decoder)> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        Ok((hdr, dec))
    }

    /// Parse the header and returns the decoder.
    fn parse_response_impl(&self) -> crate::Result<Decoder> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        if hdr.status() == Some(Status::Ok) {
            Ok(dec)
        } else {
            eprintln!("{}", self.parse_err_msg(hdr, dec));
            Err(crate::Error::new(exitcode::SOFTWARE))
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
                    Ok(msg) => msg,
                    Err(_) => dec.decode::<String>().unwrap_or_default(),
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
    pub fn print_response(&'d self) -> crate::Result<()>
    {
        let b: T::Resp = self.parse_response()?;
        let o = match self.opts.global_args.output_format {
            OutputFormat::Plain => b.output().context("Failed to serialize response body")?,
            OutputFormat::Json => {
                serde_json::to_string_pretty(&b).context("Failed to serialize response body")?
            }
        };
        println!("{}", o);
        Ok(())
    }
}

/// A simple wrapper for shutting down the local embedded node (for
/// the client side of the CLI).  Swallows errors and turns them into
/// eprintln logs.
///
/// TODO: We may want to change this behaviour in the future.
pub async fn stop_node(mut ctx: ockam::Context) -> anyhow::Result<()> {
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
/// **IMPORTANT** every handler is responsibly for shutting down its
/// local node after it's done communicating with the remote node via
/// `ctx.stop().await`!
pub fn connect_to<A, F, Fut>(port: u16, a: A, lambda: F)
where
    A: Send + Sync + 'static,
    F: FnOnce(ockam::Context, A, Route) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<()>> + Send + 'static,
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

fn node_addr(cfg: &NodeConfig) -> Address {
    Address::from((TCP, format!("localhost:{}", cfg.port)))
}

pub fn node_rpc<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(ockam::Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = crate::Result<()>> + Send + 'static,
{
    let res = embedded_node(
        |ctx, a| async {
            let res = f(ctx, a).await;
            if let Err(e) = res {
                std::process::exit(e.code());
            }
            Ok(())
        },
        a,
    );
    if let Err(e) = res {
        eprintln!("Ockam node failed: {:?}", e);
        std::process::exit(exitcode::SOFTWARE);
    }
}

pub fn embedded_node<A, F, Fut, T>(f: F, a: A) -> anyhow::Result<T>
where
    A: Send + Sync + 'static,
    F: FnOnce(ockam::Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();
    executor
        .execute(async move {
            match f(ctx, a).await {
                Err(e) => {
                    eprintln!("Error {:?}", e);
                    std::process::exit(1);
                }
                Ok(v) => v,
            }
        })
        .map_err(anyhow::Error::from)
}

pub fn find_available_port() -> anyhow::Result<u16> {
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
