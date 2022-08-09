pub mod api;
pub mod startup;

mod addon;
pub use addon::AddonCommand;

mod config;
pub use config::*;

use anyhow::Context;
use ockam::{route, NodeBuilder, Route, TcpTransport, TCP};
use std::{env, net::TcpListener, path::Path};
use tracing::error;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

pub const DEFAULT_CLOUD_ADDRESS: &str = "/dnsaddr/cloud.ockam.io/tcp/62526";

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
    embedded_node(
        move |ctx, a| async move {
            let tcp = match TcpTransport::create(&ctx).await {
                Ok(tcp) => tcp,
                Err(e) => {
                    eprintln!("failed to create TcpTransport");
                    error!(%e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = tcp.connect(format!("localhost:{}", port)).await {
                eprintln!("failed to connect to node");
                error!(%e);
                std::process::exit(1);
            }
            let route = route![(TCP, format!("localhost:{}", port))];
            if let Err(e) = lambda(ctx, a, route).await {
                eprintln!("encountered an error in command handler code");
                error!(%e);
                std::process::exit(1);
            }
            Ok(())
        },
        a,
    )
}

pub fn embedded_node<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(ockam::Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();
    let res = executor.execute(async move {
        if let Err(e) = f(ctx, a).await {
            eprintln!("Error {:?}", e);
            std::process::exit(1);
        }
    });
    if let Err(e) = res {
        eprintln!("Ockam node failed: {:?}", e,);
    }
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
