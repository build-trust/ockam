use std::env;
use std::net::{SocketAddrV4, SocketAddrV6};

use tracing::error;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

use ockam::{Address, Route, TCP};
use ockam_core::LOCAL;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Ockam, Tcp};
use ockam_multiaddr::{MultiAddr, Protocol};

pub fn embedded_node<A, F, Fut>(f: F, a: A)
where
    A: Send + Sync + 'static,
    F: FnOnce(ockam::Context, A) -> Fut + Send + Sync + 'static,
    Fut: core::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (ctx, mut executor) = ockam::start_node();
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

/// Try to convert a multi-address to an Ockam route.
pub(crate) fn multiaddr_to_route(ma: &MultiAddr) -> Option<Route> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();
    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                let ip4 = p.cast::<Ip4>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV4::new(*ip4, *tcp).to_string());
                rb = rb.append(add)
            }
            Ip6::CODE => {
                let ip6 = p.cast::<Ip6>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV6::new(*ip6, *tcp, 0, 0).to_string());
                rb = rb.append(add)
            }
            DnsAddr::CODE => {
                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let tcp = p.cast::<Tcp>()?;
                        rb = rb.append(Address::new(TCP, format!("{}:{}", &*host, *tcp)));
                        let _ = it.next();
                        continue;
                    }
                }
                rb = rb.append(Address::new(TCP, &*host))
            }
            Ockam::CODE => {
                let local = p.cast::<Ockam>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_command", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(rb.into())
}

pub fn setup_logging(verbose: u8) {
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
    let filter = match env::var("OCKAM_LOG") {
        Ok(s) if !s.is_empty() => builder.with_env_var("OCKAM_LOG").from_env_lossy(),
        _ => match verbose {
            0 => builder
                .with_default_directive(LevelFilter::WARN.into())
                .parse_lossy(ockam_crates.map(|c| format!("{c}=info")).join(",")),
            1 => builder
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy(""),
            2 => builder
                .with_default_directive(LevelFilter::DEBUG.into())
                .parse_lossy(""),
            _ => builder
                .with_default_directive(LevelFilter::TRACE.into())
                .parse_lossy(""),
        },
    };

    let result = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .with(fmt::layer())
        .try_init();
    if result.is_err() {
        tracing::warn!("Failed to initialise logging.");
    }
}
