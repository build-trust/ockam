#![allow(unused)] // FIXME
// If `pipe` is so great, why is there no `pipe2`?
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
    pipe::receiver,
    Context, OckamMessage, SystemBuilder, SystemHandler,
};
use ockam_core::{
    compat::{boxed::Box, string::String, vec::Vec},
    Address, Result, Route,
};

const CLUSTER_NAME: &str = "_internal.pipe2";

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
    send_hooks: SystemBuilder<Context, OckamMessage>,
    recv_hooks: SystemBuilder<Context, OckamMessage>,
    /// The selected pipe initialisation mode
    mode: Mode,
    /// Peer information
    peer: Option<Route>,
    /// Receiver information
    recv: Option<Address>,
    /// This address is used to handle the termination point of the
    /// worker system pipeline.
    fin: Address,
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
            send_hooks: SystemBuilder::new(),
            recv_hooks: SystemBuilder::new(),
            peer: None,
            recv: None,
            fin: Address::random(0),
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
        // A pipe can currently only have two types of hooks.
        // "Delivery" and "Ordering".  Because we want "ordering" to
        // be applied _before_ "delivery" we need to make sure that if
        // "delivery" has already been added, we point towards _its_
        // address, instead of "fin".
        let next_recv_addr = self
            .recv_hooks
            .get_addr("delivery")
            .unwrap_or_else(|| self.fin.clone());
        let next_send_addr = self
            .send_hooks
            .get_addr("delivery")
            .unwrap_or_else(|| self.fin.clone());

        // Then we simply add a single SystemHandler stage
        self.recv_hooks
            .add(Address::random(0), "ordering", ReceiverOrdering::default())
            .default(next_recv_addr);
        self.send_hooks
            .add(Address::random(0), "ordering", SenderOrdering::default())
            .default(next_send_addr);
        self
    }

    /// Enable the delivery guarantee behaviour on this pipe
    ///
    /// Additional behaviours can be added to compose a custom pipe
    /// worker.
    pub fn delivery_ack(mut self) -> Self {
        self.send_hooks
            .add(Address::random(0), "delivery", SenderConfirm::default())
            .default(self.fin.clone());
        self.recv_hooks
            .add(Address::random(0), "delivery", ReceiverConfirm::default())
            .default(self.fin.clone());
        self
    }

    /// Consume this builder and construct a set of pipes
    pub async fn build(self, ctx: &mut Context) -> Result<BuilderResult> {
        let mut tx_addr = None;
        let mut rx_addr = None;

        // Create a sender
        if let Some(peer) = self.peer {
            let (addr, int_addr) = (Address::random(0), Address::random(0));
            let sender = sender::PipeSender::new(peer, addr.clone(), int_addr.clone());
            ctx.start_worker(vec![addr.clone(), int_addr], sender)
                .await?;
            tx_addr = Some(addr);
        };

        if let Some(addr) = self.recv {
            let receiver = receiver::PipeReceiver::new(Address::random(0));
            ctx.start_worker(addr.clone(), receiver).await?;
            rx_addr = Some(addr);
        }

        Ok(BuilderResult {
            tx: tx_addr,
            rx: rx_addr,
        })
    }
}

#[crate::test]
async fn very_simple_pipe2(ctx: &mut Context) -> Result<()> {
    let rx_addr = Address::random(0);
    let tx_addr = Address::random(0);

    // Start a static receiver
    PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .build(ctx)
        .await?;

    // Connect to a static receiver
    let sender = PipeBuilder::fixed()
        .connect(vec![rx_addr])
        .build(ctx)
        .await?;

    let msg = String::from("Hello through the pipe");
    ctx.send(vec![sender.addr(), "app".into()], msg.clone())
        .await?;

    let msg2 = ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}
