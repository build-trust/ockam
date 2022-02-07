use crate::{Context, Executor};
use ockam_core::{Address, Passthrough};
use tracing_subscriber::prelude::*;
#[cfg(feature = "std")]
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

/// A minimal worker implementation that does nothing
pub struct NullWorker;

impl ockam_core::Worker for NullWorker {
    type Context = Context;
    type Message = (); // This message type is never used
}

/// Start a node
pub fn start_node() -> (Context, Executor) {
    setup_tracing();

    info!("Initializing ockam node");

    let mut exe = Executor::new();
    let addr: Address = "app".into();

    // The root application worker needs a mailbox and relay to accept
    // messages from workers, and to buffer incoming transcoded data.
    let (ctx, sender, _) = Context::new(exe.runtime(), exe.sender(), addr.into(), Passthrough);

    // Register this mailbox handle with the executor
    exe.initialize_system("app", sender);

    (ctx, exe)
}

/// Utility to setup tracing-subscriber from the environment
#[cfg(not(feature = "console"))]
fn setup_tracing() {
    #[cfg(feature = "std")]
    {
        let filter = Box::new(EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
            EnvFilter::default()
                .add_directive(LevelFilter::INFO.into())
                .add_directive("ockam_node=info".parse().unwrap())
        }));

        if tracing_subscriber::registry()
            .with(filter)
            .try_init()
            .is_err()
        {
            debug!("Failed to initialise tracing_subscriber.  Is an instance already running?");
        }
    }
}

#[cfg(feature = "console")]
/// Utility to setup tracing-subscriber from the environment.
/// Enables usage with the `tokio-console` diagnostic toolkit.
///
/// # Usage
/// The `tokio_unstable` cfg flag, which enables experimental APIs in Tokio, must
/// be enabled. It can be enabled by setting the `RUSTFLAGS` environment variable
/// at build-time:
/// ```shell
/// $ RUSTFLAGS="--cfg tokio_unstable" cargo build
/// ```
///
fn setup_tracing() {
    #[cfg(feature = "std")]
    {
        let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
            EnvFilter::default()
                .add_directive(LevelFilter::INFO.into())
                .add_directive("ockam_node=info".parse().unwrap())
                .add_directive("tokio=trace".parse().unwrap())
                .add_directive("runtime=trace".parse().unwrap())
        });

        let console_layer = console_subscriber::spawn();

        if tracing_subscriber::registry()
            .with(console_layer)
            .with(filter)
            .try_init()
            .is_err()
        {
            debug!("Failed to initialise tracing_subscriber.  Is an instance already running?");
        }
    }
}
