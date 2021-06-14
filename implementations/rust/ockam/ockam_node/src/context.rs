use std::{sync::Arc, time::Duration};

use ockam_core::{
    Address, AddressSet, LocalMessage, Message, Result, Route, TransportMessage, Worker,
};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Sender},
    time::timeout,
};

use crate::{
    block_future,
    error::Error,
    node::NullWorker,
    parser,
    relay::{self, RelayMessage},
    Cancel, Mailbox, NodeMessage,
};

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: u64 = 5;

/// Context contains Node state and references to the runtime.
pub struct Context {
    address: AddressSet,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
    pub(crate) mailbox: Mailbox,
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let addr = self.address.first();
        trace!("Running Context::drop()");

        if let Err(e) = block_future(self.rt.as_ref(), async { self.stop_worker(addr).await }) {
            trace!("Error occured during Context::drop(): {}", e);
        };
    }
}

impl Context {
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: AddressSet,
        mailbox: Mailbox,
    ) -> Self {
        Self {
            rt,
            sender,
            address,
            mailbox,
        }
    }

    /// Return the primary worker address
    pub fn address(&self) -> Address {
        self.address.first().clone()
    }

    /// Return all addresses of this worker
    pub fn aliases(&self) -> AddressSet {
        self.address
            .clone()
            .into_iter()
            .skip(1)
            .collect::<Vec<_>>()
            .into()
    }

    /// Create a new context without spawning a full worker
    pub async fn new_context<S: Into<Address>>(&self, addr: S) -> Result<Context> {
        let addr = addr.into();
        let ctx = NullWorker::new(Arc::clone(&self.rt), &addr, self.sender.clone());

        // Create a small relay and register it with the internal router
        let sender = relay::build_root::<NullWorker, _>(Arc::clone(&self.rt), &ctx.mailbox);
        let (msg, mut rx) = NodeMessage::start_worker(addr.into(), sender);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartWorker)?;

        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ctx)?)
    }

    /// Start a new worker handle at [`Address`](ockam_core::Address)
    pub async fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<AddressSet>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        let address = address.into();

        // Check if the address set is available
        let (check_addrs, mut check_rx) = NodeMessage::check_address(address.clone());
        self.sender
            .send(check_addrs)
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        check_rx.recv().await.ok_or(Error::InternalIOFailure)??;

        // Build the mailbox first
        let (mb_tx, mb_rx) = channel(32);
        let mb = Mailbox::new(mb_rx, mb_tx);

        // Pass it to the context
        let ctx = Context::new(self.rt.clone(), self.sender.clone(), address.clone(), mb);

        // Then initialise the worker message relay
        let sender = relay::build::<NW, NM>(self.rt.as_ref(), worker, ctx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_worker(address, sender);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartWorker)?;

        // Wait for the actual return code
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Signal to the local application runner to shut down
    pub async fn stop(&mut self) -> Result<()> {
        let tx = self.sender.clone();
        info!("Shutting down all workers");
        match tx.send(NodeMessage::StopNode).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        }
    }

    /// Shut down a worker by its primary address
    pub async fn stop_worker<A: Into<Address>>(&self, addr: A) -> Result<()> {
        let addr = addr.into();
        debug!("Shutting down worker {}", addr);

        // Send the stop request
        let (req, mut rx) = NodeMessage::stop_worker(addr);
        self.sender.send(req).await.map_err(|e| Error::from(e))?;

        // Then check that the worker was properly shut down
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Send a message via a fully qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.send_from_address(route, msg, self.address()).await
    }

    /// Send a message via a fully qualified route using specific Worker address
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockem_core::RouteBuilder
    pub async fn send_from_address<R, M>(
        &self,
        route: R,
        msg: M,
        sending_address: Address,
    ) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        if !self.address.as_ref().contains(&sending_address) {
            return Err(Error::SenderAddressDoesntExist.into());
        }

        let route = route.into();
        let (reply_tx, mut reply_rx) = channel(1);
        let next = route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(|e| Error::from(e))?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().unwrap();
        let mut transport_msg = TransportMessage::v1(route.clone(), Route::new(), payload);
        transport_msg.return_route.modify().append(sending_address);
        let local_msg = LocalMessage::new(transport_msg, Vec::new());

        // Pack transport message into relay message wrapper
        let msg = if needs_wrapping {
            RelayMessage::pre_router(addr, local_msg, route)
        } else {
            RelayMessage::direct(addr, local_msg, route)
        };

        // Send the packed user message with associated route
        sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(())
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send_message`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send_message`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`TransportMessage`]: ockam_core::TransportMessage
    pub async fn forward(&self, local_msg: LocalMessage) -> Result<()> {
        // Resolve the sender for the next hop in the messages route
        let (reply_tx, mut reply_rx) = channel(1);
        let next = local_msg.transport().onward_route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(|e| Error::from(e))?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the transport message into a relay message
        let onward = local_msg.transport().onward_route.clone();
        // let msg = RelayMessage::direct(addr, data, onward);
        let msg = if needs_wrapping {
            RelayMessage::pre_router(addr, local_msg, onward)
        } else {
            RelayMessage::direct(addr, local_msg, onward)
        };
        sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(())
    }

    /// Block the current worker to wait for a typed message
    ///
    /// This function may return a `Err(FailedLoadData)` if the
    /// underlying worker was shut down, or `Err(Timeout)` if the call
    /// was waiting for longer than the `default timeout`.  Use
    /// [`receive_timeout`](Context::receive_timeout) to adjust the
    /// timeout period.
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<'ctx, M: Message>(&'ctx mut self) -> Result<Cancel<'ctx, M>> {
        self.receive_timeout(DEFAULT_TIMEOUT).await
    }

    /// Block to wait for a typed message, with explicit timeout
    pub async fn receive_timeout<'ctx, M: Message>(
        &'ctx mut self,
        timeout_secs: u64,
    ) -> Result<Cancel<'ctx, M>> {
        let (msg, data, addr) = timeout(Duration::from_secs(timeout_secs), async {
            self.next_from_mailbox().await
        })
        .await
        .map_err(|e| Error::from(e))??;
        Ok(Cancel::new(msg, data, addr, self))
    }

    /// Block the current worker to wait for a message satisfying a conditional
    ///
    /// Will return `Err` if the corresponding worker has been
    /// stopped, or the underlying node has shut down.  This operation
    /// has a [default timeout](DEFAULT_TIMEOUT).
    ///
    /// Internally this function calls `receive` and `.cancel()` in a
    /// loop until a matching message is found.
    pub async fn receive_match<'ctx, M, F>(&'ctx mut self, check: F) -> Result<Cancel<'ctx, M>>
    where
        M: Message,
        F: Fn(&M) -> bool,
    {
        let (m, data, addr) = timeout(Duration::from_secs(DEFAULT_TIMEOUT), async {
            loop {
                match self.next_from_mailbox().await {
                    Ok((m, data, addr)) if check(&m) => break Ok((m, data, addr)),
                    Ok((_, data, addr)) => {
                        let onward = data.transport().onward_route.clone();
                        self.mailbox
                            .requeue(RelayMessage::direct(addr, data, onward))
                            .await;
                    }
                    e => break e,
                }
            }
        })
        .await
        .map_err(|e| Error::from(e))??;

        Ok(Cancel::new(m, data, addr, self))
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender.send(msg).await.map_err(|e| Error::from(e))?;

        Ok(reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_workers()?)
    }

    /// Register a router for a specific address type
    pub async fn register<A: Into<Address>>(&self, type_: u8, addr: A) -> Result<()> {
        let addr = addr.into();
        let (tx, mut rx) = channel(1);
        self.sender
            .send(NodeMessage::Router(type_, addr, tx))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        Ok(rx.recv().await.ok_or(Error::InternalIOFailure)??.is_ok()?)
    }

    /// A convenience function to get a data 3-tuple from the mailbox
    ///
    /// The reason this function doesn't construct a `Cancel<_, M>` is
    /// to avoid the lifetime collision between the mutation on `self`
    /// and the ref to `Context` passed to `Cancel::new(..)`
    ///
    /// This function will block and re-queue messages into the
    /// mailbox until it can receive the correct message payload.
    ///
    /// WARNING: this will temporarily create a busyloop, this
    /// mechanism should be replaced with a waker system that lets the
    /// mailbox work not yield another message until the relay worker
    /// has woken it.
    async fn next_from_mailbox<M: Message>(&mut self) -> Result<(M, LocalMessage, Address)> {
        loop {
            let msg = self
                .mailbox
                .next()
                .await
                .ok_or_else(|| Error::FailedLoadData)?;
            let (addr, data) = msg.local_msg();

            // FIXME: make message parsing idempotent to avoid cloning
            match parser::message(&data.transport().payload).ok() {
                Some(msg) => break Ok((msg, data, addr)),
                None => {
                    let onward = data.transport().onward_route.clone();
                    self.mailbox
                        .requeue(RelayMessage::direct(addr, data, onward))
                        .await;
                }
            }
        }
    }
}
