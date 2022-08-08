use std::{env, net::TcpListener, path::Path};

use anyhow::Context;
use minicbor::{Decode, Decoder, Encode};
use tracing::error;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub use addon::AddonCommand;
pub use config::*;
use ockam::{route, Address, NodeBuilder, Route, TcpTransport, TCP};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{RequestBuilder, Response, Status};

use crate::util::output::Output;
use crate::{CommandGlobalOpts, OutputFormat};

pub mod api;
pub mod exitcode;
pub mod startup;

mod addon;
mod config;
mod output;

pub const DEFAULT_CLOUD_ADDRESS: &str = "/dnsaddr/cloud.ockam.io/tcp/62526";

pub struct Rpc<'a> {
    ctx: &'a ockam::Context,
    buf: Vec<u8>,
    opts: &'a CommandGlobalOpts,
    cfg: NodeConfig,
}

impl<'a> Rpc<'a> {
    pub fn new(
        ctx: &'a ockam::Context,
        opts: &'a CommandGlobalOpts,
        api_node: &'a str,
    ) -> anyhow::Result<Self> {
        Ok(Rpc {
            ctx,
            buf: Vec::new(),
            opts,
            cfg: opts.config.get_node(api_node)?,
        })
    }

    pub async fn request<T>(&mut self, req: RequestBuilder<'_, T>) -> anyhow::Result<()>
    where
        T: Encode<()>,
    {
        let buf = req.to_vec()?;
        let mut rte = connect(self.ctx, &self.cfg).await?;
        self.buf = self
            .ctx
            .send_and_receive(rte.modify().append(NODEMANAGER_ADDR), buf)
            .await?;
        Ok(())
    }

    /// Parse the response body and return it.
    pub fn parse_response<T>(&'a self) -> crate::Result<T>
    where
        T: Decode<'a, ()>,
    {
        let mut dec = self.parse_response_impl()?;
        Ok(dec.decode().context("Failed to decode response body")?)
    }

    /// Parse response header only to check the status code.
    pub fn check_response(&self) -> crate::Result<()> {
        self.parse_response_impl()?;
        Ok(())
    }

    /// Parse the response body and return it.
    fn parse_response_impl(&'a self) -> crate::Result<Decoder> {
        let mut dec = Decoder::new(&self.buf);
        let hdr = dec
            .decode::<Response>()
            .context("Failed to decode response header")?;
        match hdr.status() {
            Some(Status::Ok) if hdr.has_body() => {
                return Ok(dec);
            }
            Some(Status::Ok) => {
                eprintln!("No body found in response");
            }
            Some(status) if hdr.has_body() => {
                let err = dec.decode::<String>().unwrap_or_default();
                eprintln!(
                    "An error occurred while processing the request. Status code: {status}. {err}"
                );
            }
            Some(status) => {
                eprintln!("An error occurred while processing the request. Status code: {status}",);
            }
            None => {
                eprintln!("No status found in response");
            }
        };
        Err(crate::Error::new(exitcode::SOFTWARE))
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

async fn connect(ctx: &ockam::Context, cfg: &NodeConfig) -> anyhow::Result<Route> {
    let adr = Address::from((TCP, format!("localhost:{}", cfg.port)));
    let tcp = TcpTransport::create(ctx).await?;
    tcp.connect(adr.address()).await?;
    Ok(adr.into())
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
