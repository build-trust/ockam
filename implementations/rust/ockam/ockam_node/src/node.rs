use crate::{Context, Executor};
use ockam_core::Address;
#[cfg(feature = "std")]
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

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
    let (ctx, sender) = Context::new(exe.runtime(), exe.sender(), addr.into());

    // Register this mailbox handle with the executor
    exe.initialize_system("app", sender);

    (ctx, exe)
}

/// Utility to setup tracing-subscriber from the environment
fn setup_tracing() {
    #[cfg(feature = "std")]
    {
        let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
            EnvFilter::default()
                .add_directive(LevelFilter::INFO.into())
                .add_directive("ockam_node=info".parse().unwrap())
        });

        if fmt().with_env_filter(filter).try_init().is_err() {
            debug!("Failed to initialise tracing_subscriber.  Is an instance already running?");
        }
    }
}
