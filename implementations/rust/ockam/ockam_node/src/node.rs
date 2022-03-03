use crate::{Context, Executor};
use ockam_core::{Address, AllowAll};

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
    let (ctx, sender, _) = Context::new(exe.runtime(), exe.sender(), addr.into(), AllowAll);

    // Register this mailbox handle with the executor
    exe.initialize_system("app", sender);

    (ctx, exe)
}

/// Utility to setup tracing-subscriber from the environment.
///
/// Does nothing if the `no_init_tracing` feature is enabled (for now -- this
/// should be improved, though).
fn setup_tracing() {
    #[cfg(feature = "std")]
    if !cfg!(feature = "no_init_tracing") {
        use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
                EnvFilter::default()
                    .add_directive(LevelFilter::INFO.into())
                    .add_directive("ockam_node=info".parse().unwrap())
            });
            if fmt().with_env_filter(filter).try_init().is_err() {
                debug!("Failed to initialise tracing_subscriber. Is an instance already running?");
            }
        });
    } else {
        info!("Logging auto-init disabled, assuming it's initialized separately")
    }
}
