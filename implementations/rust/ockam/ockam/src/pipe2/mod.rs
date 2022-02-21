//! Pipe2 composition system
//!
//! Pipe2 offers the ability to compose pipe workers with different
//! behaviours.  These behaviours are implemented using the
//! [`SystemHandler`](crate::SystemHandler) abstraction.
//!
//! This module is a replacement for [`pipe`](crate::pipe) and should
//! replace it at some point in the future.

mod receiver;
mod sender;

use crate::{
    hooks::pipe::{ReceiverConfirm, ReceiverOrdering, SenderConfirm, SenderOrdering},
    Context, OckamMessage, SystemBuilder, WorkerSystem,
};
use ockam_core::{
    compat::{collections::BTreeSet, string::String},
    Address, Result, Route,
};
pub use receiver::PipeReceiver;
pub use sender::PipeSender;

const CLUSTER_NAME: &str = "_internal.pipe2";
type PipeSystem = WorkerSystem<Context, OckamMessage>;

enum Mode {
    /// In static mode this pipe will connect to a well-known peer, or
    /// receive _one_ connection on a well-known address, or does
    /// both at the same time
    Static,
    /// In dynamic mode this pipe connects to a peer via an
    /// initialisation handshake, or listens for initialisation
    /// handshakes
    Dynamic,
}

/// A builder structure for pipes
///
/// A pipe is a unidirectional message sending abstraction, which
/// optionally provides ordering and delivery guarantees.  The two
/// basic pipe initialisation modes are `Fixed`, connecting to a
/// specific peer route, and `Dynamic`, connecting to a handshake
/// worker which then creates a remote peer dynamically.
///
/// ## Static example
///
/// The easiest way to get started with pipes is with a static route.
/// This requires a running `PipeReceiver` worker on a remote system.
///
/// Code on machine A:
///
/// ```rust
/// # use ockam::{Context, Result, Address, pipe2::PipeBuilder};
/// # async fn pipes_example_no_run(ctx: &mut Context) -> Result<()> {
/// # let (tcp_connection, my_pipe) = (Address::random(0), Address::random(0));
/// let result = PipeBuilder::fixed()
///     .connect(vec![tcp_connection, my_pipe])
///     .build(ctx)
///     .await?;
///
/// ctx.send(
///     vec![result.addr(), "app".into()], // Send a message through the pipe to "app"
///     String::from("Hello you on the other end of this pipe!"),
/// )
/// .await?;
/// # Ok(())
/// # }
/// ```
///
/// Code on machine B:
///
/// ```rust
/// # use ockam::{Context, Result, Address, pipe2::PipeBuilder};
/// # async fn pipes_example_no_run(ctx: &mut Context) -> Result<()> {
/// # let my_pipe = Address::random(0);
/// let receive = PipeBuilder::fixed()
///     .receive(my_pipe)
///     .build(ctx)
///     .await?;
///
/// let msg = ctx.receive::<String>().await?;
/// println!("Message from pipe: {}", msg);
/// # Ok(())
/// # }
/// ```
pub struct PipeBuilder {
    hooks: BTreeSet<PipeHook>,
    /// The selected pipe initialisation mode
    mode: Mode,
    /// Peer information
    peer: Option<Route>,
    /// Receiver information
    recv: Option<Address>,
    /// "Fin" address on the sender
    tx_fin: Address,
    /// "Fin" address on the receiver
    rx_fin: Address,
}

/// A simple wrapper around possible pipe hooks
#[derive(PartialOrd, Ord, PartialEq, Eq)]
enum PipeHook {
    Ordering,
    Delivery,
}

/// Represent the result of a successful PipeBuilder invocation
///
/// When connecting to a remote pipe receiver `tx()` returns the
/// associated sending address.  When creating a receiver `rx()`
/// returns the associated receiver address.
///
/// In case you only created one of them you may call `addr()` to
/// fetch the only valid address.  But this will panic if both
/// addresses are set!
pub struct BuilderResult {
    tx: Option<Address>,
    rx: Option<Address>,
}

impl BuilderResult {
    /// Return the sender address
    pub fn tx(&self) -> Option<&Address> {
        self.tx.as_ref()
    }

    /// Return the receiver address
    pub fn rx(&self) -> Option<&Address> {
        self.rx.as_ref()
    }

    /// Return the only valid address in this result
    ///
    /// Panics if two valid addresses exist!
    pub fn addr(&self) -> Address {
        match (&self.tx, &self.rx) {
            (Some(tx), None) => tx.clone(),
            (None, Some(rx)) => rx.clone(),
            (Some(_), Some(_)) => panic!("Called `addr()` on ambiguous BuilderResult!"),
            (None, None) => unreachable!(),
        }
    }
}

impl PipeBuilder {
    fn new(mode: Mode) -> Self {
        Self {
            hooks: BTreeSet::new(),
            peer: None,
            recv: None,
            tx_fin: Address::random(0),
            rx_fin: Address::random(0),
            mode,
        }
    }

    /// Construct a fixed pipe to a specific well-known peer
    pub fn fixed() -> Self {
        Self::new(Mode::Static)
    }

    /// Construct a pipe using dynamic initialisation handshakes
    pub fn dynamic() -> Self {
        Self::new(Mode::Dynamic)
    }

    /// Set a connection peer, creating an outgoing pipe
    ///
    /// * In `fixed` mode this attempts to connect directly to a
    /// receiver at the given peer route.
    ///
    /// * In `dynamic` mode this initiates a handshake with the
    /// given peer.  This handshake then resolves to the final
    /// receiver
    pub fn connect<R: Into<Route>>(mut self, peer: R) -> Self {
        self.peer = Some(peer.into());
        self
    }

    /// Set a receiving address, creating a receiving pipe
    ///
    /// * In `fixed` mode this creates a pipe receiver which waits
    /// for incoming messages from a sender.
    ///
    /// * In `dynamic` mode this spawns a handshake listener, which
    /// will create pipe receivers dynamically for any incoming
    /// initialisation request
    pub fn receive<A: Into<Address>>(mut self, addr: A) -> Self {
        self.recv = Some(addr.into());
        self
    }

    /// Set this pipe to enforce the ordering of incoming messages
    pub fn enforce_ordering(mut self) -> Self {
        self.hooks.insert(PipeHook::Ordering);
        self
    }

    /// Enable the delivery guarantee behaviour on this pipe
    ///
    /// Additional behaviours can be added to compose a custom pipe
    /// worker.
    pub fn delivery_ack(mut self) -> Self {
        self.hooks.insert(PipeHook::Delivery);
        self
    }

    async fn build_systems(&self, ctx: &mut Context) -> Result<(PipeSystem, PipeSystem)> {
        let mut send_hooks = SystemBuilder::new();
        let mut recv_hooks = SystemBuilder::new();

        let (ord_tx_addr, ord_rx_addr) = (Address::random(0), Address::random(0));
        let (ack_tx_addr, ack_rx_addr) = (Address::random(0), Address::random(0));

        // Setup ordering enforcement hooks
        if self.hooks.contains(&PipeHook::Ordering) {
            // Setup the sender ordering hook
            send_hooks
                .add(ord_tx_addr.clone(), "ordering", SenderOrdering::default())
                .default(if self.hooks.contains(&PipeHook::Delivery) {
                    ack_tx_addr.clone()
                } else {
                    self.tx_fin.clone()
                });

            // Setup the receiver ordering hook
            recv_hooks
                .add(ord_rx_addr.clone(), "ordering", ReceiverOrdering::default())
                .default(self.rx_fin.clone());
        }

        // Setup delivery confirmation hooks
        if self.hooks.contains(&PipeHook::Delivery) {
            send_hooks
                .add(ack_tx_addr.clone(), "delivery", SenderConfirm::default())
                .default(self.tx_fin.clone());

            recv_hooks
                .add(ack_rx_addr.clone(), "delivery", ReceiverConfirm::default())
                .default(if self.hooks.contains(&PipeHook::Ordering) {
                    ord_rx_addr.clone()
                } else {
                    self.rx_fin.clone()
                });
        }

        Ok((
            send_hooks.finalise(ctx).await?,
            recv_hooks.finalise(ctx).await?,
        ))
    }

    async fn build_fixed(
        self,
        ctx: &mut Context,
        tx_sys: PipeSystem,
        rx_sys: PipeSystem,
    ) -> Result<BuilderResult> {
        let mut tx_addr = None;
        let mut rx_addr = None;

        // Create a sender
        if let Some(peer) = self.peer {
            let (addr, int_addr) = (Address::random(0), Address::random(0));
            let sender = PipeSender::new(tx_sys, peer, addr.clone(), int_addr.clone());
            ctx.start_worker(vec![addr.clone(), int_addr, self.tx_fin.clone()], sender)
                .await?;
            tx_addr = Some(addr);
        };

        // Create a receiver
        if let Some(addr) = self.recv {
            let receiver = PipeReceiver::new(rx_sys, Address::random(0));
            ctx.start_worker(vec![addr.clone(), self.rx_fin.clone()], receiver)
                .await?;
            rx_addr = Some(addr);
        }

        Ok(BuilderResult {
            tx: tx_addr,
            rx: rx_addr,
        })
    }

    async fn build_dynamic(
        self,
        ctx: &mut Context,
        tx_sys: PipeSystem,
        rx_sys: PipeSystem,
    ) -> Result<BuilderResult> {
        todo!()
    }

    /// Consume this builder and construct a set of pipes
    pub async fn build(self, ctx: &mut Context) -> Result<BuilderResult> {
        let (tx_sys, rx_sys) = self.build_systems(ctx).await?;

        match self.mode {
            Mode::Static => self.build_fixed(ctx, tx_sys, rx_sys).await,
            Mode::Dynamic => self.build_dynamic(ctx, tx_sys, rx_sys).await,
        }
    }
}

#[crate::test]
async fn very_simple_pipe2(ctx: &mut Context) -> Result<()> {
    let rx_addr = Address::random(0);

    // Start a static receiver
    let rx = PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .build(ctx)
        .await?;
    info!("Created receiver pipe: {}", rx.addr());

    // Connect to a static receiver
    let sender = PipeBuilder::fixed()
        .connect(vec![rx_addr])
        .build(ctx)
        .await?;
    info!("Created sender pipe: {}", sender.addr());

    let msg = String::from("Hello through the pipe");
    ctx.send(vec![sender.addr(), "app".into()], msg.clone())
        .await?;

    let msg2 = ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}
