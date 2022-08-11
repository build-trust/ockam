use crate::{debugger, Context, Executor};
use ockam_core::compat::sync::Arc;
use ockam_core::{AccessControl, Address, AllowAll, AllowAllOutgoing, Mailbox, Mailboxes};

/// A minimal worker implementation that does nothing
pub struct NullWorker;

impl ockam_core::Worker for NullWorker {
    type Context = Context;
    type Message = (); // This message type is never used
}

/// Start a node with a custom setup configuration
///
/// The `start_node()` function wraps this type and simply calls
/// `NodeBuilder::default()`.  Varying use-cases should use the
/// builder API to customise the underlying node that is created.
pub struct NodeBuilder<AC>
where
    AC: AccessControl,
{
    access_control: AC,
    logging: bool,
}

impl NodeBuilder<AllowAll> {
    /// Create a node with "AllowAll" access control
    pub fn without_access_control() -> Self {
        Self {
            access_control: AllowAll,
            logging: true,
        }
    }
}

impl<AC> NodeBuilder<AC>
where
    AC: AccessControl,
{
    /// Create a node with custom access control
    pub fn with_access_control(access_control: AC) -> Self {
        Self {
            access_control,
            logging: true,
        }
    }

    /// Disable logging on this node
    pub fn no_logging(self) -> Self {
        Self {
            logging: false,
            ..self
        }
    }

    /// Consume this builder and yield a new Ockam Node
    #[inline]
    pub fn build(self) -> (Context, Executor) {
        if self.logging {
            setup_tracing();
        }

        info!(
            "Initializing ockam node with access control: {:?}",
            self.access_control
        );

        let mut exe = Executor::new();
        let addr: Address = "app".into();

        // The root application worker needs a mailbox and relay to accept
        // messages from workers, and to buffer incoming transcoded data.
        let incoming = Arc::new(self.access_control);
        let outgoing = Arc::new(AllowAllOutgoing); // TODO
        let (ctx, sender, _) = Context::new(
            exe.runtime().clone(),
            exe.sender(),
            Mailboxes::new(Mailbox::new(addr, incoming, outgoing), vec![]),
            None,
        );

        debugger::log_inherit_context("NODE", &ctx, &ctx);

        // Register this mailbox handle with the executor
        exe.initialize_system("app", sender);

        // Then return the root context and executor
        (ctx, exe)
    }
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
