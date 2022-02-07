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

mod hooks;
mod receiver;
mod sender;

use crate::{Context, OckamMessage, SystemBuilder, SystemHandler};
use ockam_core::{
    compat::{boxed::Box, string::String, vec::Vec},
    Address, Result, Route,
};

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
/// The easiest way to create a pipe is via
/// `builder.basic().static()`.
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
            .add(
                Address::random(0),
                "ordering",
                hooks::ReceiverOrdering::default(),
            )
            .default(next_recv_addr);
        self.send_hooks
            .add(
                Address::random(0),
                "ordering",
                hooks::SenderOrdering::default(),
            )
            .default(next_send_addr);
        self
    }

    /// Enable the delivery guarantee behaviour on this pipe
    ///
    /// Additional behaviours can be added to compose a custom pipe
    /// worker.
    pub fn delivery_ack(mut self) -> Self {
        self.send_hooks
            .add(
                Address::random(0),
                "delivery",
                hooks::SenderConfirm::default(),
            )
            .default(self.fin.clone());
        self.recv_hooks
            .add(
                Address::random(0),
                "delivery",
                hooks::ReceiverConfirm::default(),
            )
            .default(self.fin.clone());
        self
    }

    /// Consume this builder and construct a set of pipes
    pub async fn build(self, ctx: &mut Context) -> Result<()> {
        Ok(())
    }
}
