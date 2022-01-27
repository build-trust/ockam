use crate::relay::RelayPayload;
use crate::tokio::{
    self,
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};
use crate::{
    error::Error,
    parser,
    relay::{CtrlSignal, ProcessorRelay, RelayMessage, WorkerRelay},
    router::SenderPair,
    Cancel, NodeMessage, ShutdownType,
};
use crate::{NodeError, Reason};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, string::String, sync::Arc, vec::Vec};
use ockam_core::{
    AccessControl, Address, AddressSet, LocalMessage, Message, Passthrough, Processor, Result,
    Route, TransportMessage, Worker,
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
    access_control: Box<dyn AccessControl>,
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }
    /// Wait for the next message from the mailbox
    pub(crate) async fn mailbox_next(&mut self) -> Result<Option<RelayMessage>> {
        loop {
            let relay_msg;
            if let Some(msg) = self.mailbox.recv().await.map(|msg| {
                trace!("{}: received new message!", self.address());
                msg
            }) {
                relay_msg = msg;
            } else {
                return Ok(None);
            }

            if let RelayPayload::Direct(local_msg) = &relay_msg.data {
                if !self.access_control.msg_is_authorized(local_msg).await? {
                    warn!("Message for {} did not pass access control", relay_msg.addr);
                    continue;
                }
            }

            return Ok(Some(relay_msg));
        }
    }
}

impl Context {
    /// Create a new context
    ///
    /// This function returns a new instance of Context, the relay
    /// sender pair, and relay control signal receiver.
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: AddressSet,
        access_control: impl AccessControl,
    ) -> (Self, SenderPair, Receiver<CtrlSignal>) {
        let (mailbox_tx, mailbox) = channel(32);
        let (ctrl_tx, ctrl_rx) = channel(1);
        (
            Self {
                rt,
                sender,
                address,
                mailbox,
                access_control: Box::new(access_control),
            },
            SenderPair {
                msgs: mailbox_tx,
                ctrl: ctrl_tx,
            },
            ctrl_rx,
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
        let (ctx, sender, _) = Self::new(
            Arc::clone(&self.rt),
            self.sender.clone(),
            addr.clone().into(),
            Passthrough,
        );

        // Create a "bare relay" and register it with the router
        let (msg, mut rx) = NodeMessage::start_worker(addr.into(), sender, true);
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
        self.start_worker_impl(address.into(), worker, Passthrough)
            .await
    }

    /// TODO: Worker builder?
    pub async fn start_worker_with_access_control<NM, NW, NA, S>(
        &self,
        address: S,
        worker: NW,
        access_control: NA,
    ) -> Result<()>
    where
        S: Into<AddressSet>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
        NA: AccessControl,
    {
        self.start_worker_impl(address.into(), worker, access_control)
            .await
    }

    async fn start_worker_impl<NM, NW, NA>(
        &self,
        address: AddressSet,
        worker: NW,
        access_control: NA,
    ) -> Result<()>
    where
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
        NA: AccessControl,
    {
        // Pass it to the context
        let (ctx, sender, ctrl_rx) = Context::new(
            self.rt.clone(),
            self.sender.clone(),
            address.clone(),
            access_control,
        );

        // Then initialise the worker message relay
        WorkerRelay::<NW, NM>::init(self.rt.as_ref(), worker, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_worker(address, sender, false);
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
        let addr = address.clone();

        let (ctx, senders, ctrl_rx) = Context::new(
            self.rt.clone(),
            self.sender.clone(),
            addr.into(),
            Passthrough,
        );

        // Initialise the processor relay with the ctrl receiver
        ProcessorRelay::<P>::init(self.rt.as_ref(), processor, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_processor(address, senders);
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

    /// Signal to the local runtime to shut down immediately
    ///
    /// **WARNING**: calling this function may result in data loss.
    /// It is recommended to use the much safer
    /// [`Context::stop`](Context::stop) function instead!
    pub async fn stop_now(&mut self) -> Result<()> {
        let tx = self.sender.clone();
        info!("Immediately shutting down all workers");
        let (msg, _) = NodeMessage::stop_node(ShutdownType::Immediate);

        match tx.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        }
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed.
    /// The default timeout for a safe shutdown is 1 second.  You can
    /// change this behaviour by calling
    /// [`Context::stop_timeout`](Context::stop_timeout) directly.
    pub async fn stop(&mut self) -> Result<()> {
        self.stop_timeout(1).await
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed
    /// or the desired timeout has been reached.
    pub async fn stop_timeout(&mut self, seconds: u8) -> Result<()> {
        let (req, mut rx) = NodeMessage::stop_node(ShutdownType::Graceful(seconds));
        self.sender.send(req).await.map_err(Error::from)?;

        // Wait until we get the all-clear
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Send a message to another address associated with this worker
    ///
    /// This function is a simple wrapper around `Self::send()` which
    /// validates the address given to it and will reject invalid
    /// addresses.
    pub async fn send_to_self<A, M>(&self, from: A, addr: A, msg: M) -> Result<()>
    where
        A: Into<Address>,
        M: Message + Send + 'static,
    {
        let addr = addr.into();
        if self.address.contains(&addr) {
            self.send_from_address(addr, msg, from.into()).await
        } else {
            Err(NodeError::Rejected(Reason::InvalidAddress).into())
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
            return Err(Error::SenderAddressDoesNotExist.into());
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

    /// Assign the current worker to a cluster
    ///
    /// A cluster is a set of workers that should be stopped together
    /// when the node is stopped or parts of the system are reloaded.
    /// **This is not to be confused with supervisors!**
    ///
    /// By adding your worker to a cluster you signal to the runtime
    /// that your worker may be depended on by other workers that
    /// should be stopped first.
    ///
    /// **Your cluster name MUST NOT start with `_internals.` or
    /// `ockam.`!**
    ///
    /// Clusters are de-allocated in reverse order of their
    /// initialisation when the node is stopped.
    pub async fn set_cluster<S: Into<String>>(&self, label: S) -> Result<()> {
        let (msg, mut rx) = NodeMessage::set_cluster(self.address(), label.into());
        self.sender.send(msg).await.map_err(Error::from)?;
        Ok(rx.recv().await.ok_or(Error::InternalIOFailure)??.is_ok()?)
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

    /// Send a shutdown acknowledgement to the router
    pub(crate) async fn send_stop_ack(&self) -> Result<()> {
        self.sender
            .send(NodeMessage::StopAck(self.address()))
            .await
            .map_err(Error::from)?;
        Ok(())
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
            let msg = self.mailbox_next().await?.ok_or(Error::FailedLoadData)?;
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

    /// Set access control for current context
    pub async fn set_access_control(&mut self) -> Result<()> {
        unimplemented!()
    }
}
