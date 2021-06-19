use crate::{relay, Context, Executor, Mailbox, NodeMessage};
use ockam_core::Address;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

/// A minimal worker implementation that does nothing
pub struct NullWorker;

impl NullWorker {
    /// Create and register a new NullWorker context
    pub(crate) fn new(rt: Arc<Runtime>, addr: &Address, tx: Sender<NodeMessage>) -> Context {
        // Create a new Mailbox and Context
        let (mb_tx, mb_rx) = channel(32);
        let mb = Mailbox::new(mb_rx, mb_tx);

        Context::new(rt, tx, addr.into(), mb)
    }
}

impl ockam_core::Worker for NullWorker {
    type Context = Context;
    type Message = (); // This message type is never used
}

/// Start a node
pub fn start_node() -> (Context, Executor) {
    setup_tracing();

    info!("Initializing ockam node");

    let mut exe = Executor::new();
    let addr = "app".into();

    // The root application worker needs a mailbox and relay to accept
    // messages from workers, and to buffer incoming transcoded data.
    let ctx = NullWorker::new(exe.runtime(), &addr, exe.sender());

    // Build a mailbox worker to buffer messages
    let sender = relay::build_root::<NullWorker, _>(exe.runtime(), &ctx.mailbox);

    // Register this mailbox handle with the executor
    exe.initialize_system("app", sender);

    (ctx, exe)
}

/// Utility to setup tracing-subscriber from the environment
fn setup_tracing() {
    let filter = EnvFilter::try_from_env("OCKAM_LOG").unwrap_or_else(|_| {
        EnvFilter::default()
            .add_directive(LevelFilter::INFO.into())
            .add_directive("ockam_node=info".parse().unwrap())
    });

    if fmt().with_env_filter(filter).try_init().is_err() {
        debug!("Failed to initialise tracing_subscriber.  Is an instance already running?");
    }
}
