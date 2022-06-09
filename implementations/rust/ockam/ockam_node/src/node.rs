use crate::{Context, Executor};
use ockam_core::compat::sync::Arc;
use ockam_core::{AccessControl, Address, AllowAll, Mailbox, Mailboxes};

/// A minimal worker implementation that does nothing
pub struct NullWorker;

impl ockam_core::Worker for NullWorker {
    type Context = Context;
    type Message = (); // This message type is never used
}

/// Start a node with [`AllowAll`] access control
pub fn start_node() -> (Context, Executor) {
    setup_tracing();
    start_node_with_access_control(AllowAll)
}

/// Start a node with [`AllowAll`] access control and no logging
pub fn start_node_without_logging() -> (Context, Executor) {
    start_node_without_logging_with_access_control(AllowAll)
}

/// Start a node with the given access control
pub fn start_node_with_access_control<AC>(access_control: AC) -> (Context, Executor)
where
    AC: AccessControl,
{
    info!(
        "Initializing ockam node with access control: {:?}",
        access_control
    );

    setup_tracing();
    start_node_without_logging_with_access_control(access_control)
}

/// Start a node but without logging
pub fn start_node_without_logging_with_access_control<AC>(access_control: AC) -> (Context, Executor)
where
    AC: AccessControl,
{
    let mut exe = Executor::new();
    let addr: Address = "app".into();

    // The root application worker needs a mailbox and relay to accept
    // messages from workers, and to buffer incoming transcoded data.
    let (ctx, sender, _) = Context::new(
        exe.runtime(),
        exe.sender(),
        Mailboxes::new(Mailbox::new(addr, Arc::new(access_control)), vec![]),
        None,
    );

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
    {
        use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
                EnvFilter::default()
                    .add_directive(LevelFilter::INFO.into())
                    .add_directive("ockam_node=info".parse().unwrap())
            });
            // Ignore failure, since we may init externally.
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(tracing_error::ErrorLayer::default())
                .with(fmt::layer())
                .try_init();
        });
    }
}
