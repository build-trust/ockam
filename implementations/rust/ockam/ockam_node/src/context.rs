use crate::relay::{ProcessorRelay, WorkerRelay};
use crate::tokio::{
    self,
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};
use crate::{
    error::Error,
    parser,
    relay::{RelayMessage, PROC_ADDR_SUFFIX},
    Cancel, NodeMessage,
};
use core::time::Duration;
use ockam_core::compat::{sync::Arc, vec::Vec};
use ockam_core::{
    Address, AddressSet, LocalMessage, Message, Processor, Result, Route, TransportMessage, Worker,
};

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: u64 = 30;

enum AddressType {
    Worker,
    Processor,
}

impl AddressType {
    fn str(&self) -> &'static str {
        match self {
            AddressType::Worker => "worker",
            AddressType::Processor => "processor",
        }
    }
}

/// Context contains Node state and references to the runtime.
pub struct Context {
    address: AddressSet,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
    mailbox: Receiver<RelayMessage>,
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }
    /// Wait for the next message from the mailbox
    pub(crate) async fn mailbox_next(&mut self) -> Option<RelayMessage> {
        self.mailbox.recv().await
    }
}

impl Context {
    /// Create a new context returning itself and the associated mailbox sender
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: AddressSet,
    ) -> (Self, Sender<RelayMessage>) {
        let (mb_tx, mailbox) = channel(32);
        (
            Self {
                rt,
                sender,
                address,
                mailbox,
            },
            mb_tx,
        )
    }

    /// Return the primary worker address
    pub fn address(&self) -> Address {
        self.address.first()
    }

    /// Return all addresses of this worker
    pub fn aliases(&self) -> AddressSet {
        self.address.clone().into_iter().skip(1).collect()
    }

    /// Utility function to sleep tasks from other crates
    #[doc(hidden)]
    pub async fn sleep(&self, dur: Duration) {
        tokio::time::sleep(dur).await;
    }

    /// Create a new context without spawning a full worker
    pub async fn new_context<S: Into<Address>>(&self, addr: S) -> Result<Context> {
        self.new_context_impl(addr.into()).await
    }

    async fn new_context_impl(&self, addr: Address) -> Result<Context> {
        // Create a new context and get access to the mailbox senders
        let (ctx, sender) = Self::new(
            Arc::clone(&self.rt),
            self.sender.clone(),
            addr.clone().into(),
        );

        // Create a small relay and register it with the internal router
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
        self.start_worker_impl(address.into(), worker).await
    }

    async fn start_worker_impl<NM, NW>(&self, address: AddressSet, worker: NW) -> Result<()>
    where
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        // Check if the address set is available
        // TODO: There is not much sense of checking for address collisions here, since in
        // async environment there may be new Workers started between the check and actual adding
        // of this Worker to the Router map, so check only should happen during Router::start_worker
        let (check_addrs, mut check_rx) = NodeMessage::check_address(address.clone());
        self.sender
            .send(check_addrs)
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        check_rx.recv().await.ok_or(Error::InternalIOFailure)??;

        // Pass it to the context
        let (ctx, sender) = Context::new(self.rt.clone(), self.sender.clone(), address.clone());

        // Then initialise the worker message relay
        WorkerRelay::<NW, NM>::init(self.rt.as_ref(), worker, ctx);

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

    /// Start a new processor at [`Address`](ockam_core::Address)
    pub async fn start_processor<P>(&self, address: impl Into<Address>, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        self.start_processor_impl(address.into(), processor).await
    }

    async fn start_processor_impl<P>(&self, address: Address, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        let main_addr = address.clone();
        let aux_addr = main_addr.suffix(PROC_ADDR_SUFFIX);

        // We create two contexts for the processor.  One is used for
        // the external representation of the processor worker
        // (i.e. sending messages, receiving messages, etc), while the
        // other is used by the router to signal shutdown conditions.
        let (main, main_tx) = Context::new(self.rt.clone(), self.sender.clone(), main_addr.into());
        let (aux, aux_tx) = Context::new(self.rt.clone(), self.sender.clone(), aux_addr.into());

        // Initialise the processor relay with the two contexts
        ProcessorRelay::<P>::init(self.rt.as_ref(), processor, main, aux);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_processor(address, main_tx, aux_tx);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartProcessor)?;

        // Wait for the actual return code
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Shut down a worker by its primary address
    pub async fn stop_worker<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Worker).await
    }

    /// Shut down a processor by its address
    pub async fn stop_processor<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Processor).await
    }

    async fn stop_address(&self, addr: Address, t: AddressType) -> Result<()> {
        debug!("Shutting down {} {}", t.str(), addr);

        // Send the stop request
        let (req, mut rx) = match t {
            AddressType::Worker => NodeMessage::stop_worker(addr),
            AddressType::Processor => NodeMessage::stop_processor(addr),
        };
        self.sender.send(req).await.map_err(Error::from)?;

        // Then check that address was properly shut down
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
        self.send_from_address(route.into(), msg, self.address())
            .await
    }

    /// Send a message via a fully qualified route using specific Worker address
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
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
        self.send_from_address_impl(route.into(), msg, sending_address)
            .await
    }

    async fn send_from_address_impl<M>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
    {
        if !self.address.as_ref().contains(&sending_address) {
            return Err(Error::SenderAddressDoesntExist.into());
        }

        let (reply_tx, mut reply_rx) = channel(1);
        let next = route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(Error::from)?;
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
        sender.send(msg).await.map_err(Error::from)?;

        Ok(())
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
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
        self.sender.send(req).await.map_err(Error::from)?;
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
        sender.send(msg).await.map_err(Error::from)?;

        Ok(())
    }

    /// Receive a message without a timeout
    pub async fn receive_block<M: Message>(&mut self) -> Result<Cancel<'_, M>> {
        let (msg, data, addr) = self.next_from_mailbox().await?;
        Ok(Cancel::new(msg, data, addr, self))
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
    pub async fn receive<M: Message>(&mut self) -> Result<Cancel<'_, M>> {
        self.receive_timeout(DEFAULT_TIMEOUT).await
    }

    /// Block to wait for a typed message, with explicit timeout
    pub async fn receive_timeout<M: Message>(
        &mut self,
        timeout_secs: u64,
    ) -> Result<Cancel<'_, M>> {
        let (msg, data, addr) = timeout(Duration::from_secs(timeout_secs), async {
            self.next_from_mailbox().await
        })
        .await
        .map_err(Error::from)??;
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
    pub async fn receive_match<M, F>(&mut self, check: F) -> Result<Cancel<'_, M>>
    where
        M: Message,
        F: Fn(&M) -> bool,
    {
        let (m, data, addr) = timeout(Duration::from_secs(DEFAULT_TIMEOUT), async {
            loop {
                match self.next_from_mailbox().await {
                    Ok((m, data, addr)) if check(&m) => break Ok((m, data, addr)),
                    Ok((_, data, _)) => {
                        // Requeue
                        self.forward(data).await?;
                    }
                    e => break e,
                }
            }
        })
        .await
        .map_err(Error::from)??;

        Ok(Cancel::new(m, data, addr, self))
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender.send(msg).await.map_err(Error::from)?;

        Ok(reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_workers()?)
    }

    /// Register a router for a specific address type
    pub async fn register<A: Into<Address>>(&self, type_: u8, addr: A) -> Result<()> {
        self.register_impl(type_, addr.into()).await
    }

    async fn register_impl(&self, type_: u8, addr: Address) -> Result<()> {
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
            let msg = self.mailbox_next().await.ok_or(Error::FailedLoadData)?;
            let (addr, data) = msg.local_msg();

            // FIXME: make message parsing idempotent to avoid cloning
            match parser::message(&data.transport().payload).ok() {
                Some(msg) => break Ok((msg, data, addr)),
                None => {
                    // Requeue
                    self.forward(data).await?;
                }
            }
        }
    }
}
