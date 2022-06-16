use crate::async_drop::AsyncDrop;
use crate::channel_types::{message_channel, small_channel, SmallReceiver, SmallSender};
use crate::tokio::{self, runtime::Runtime, time::timeout};
use crate::{
    error::*,
    parser,
    relay::{CtrlSignal, ProcessorRelay, RelayMessage},
    router::SenderPair,
    Cancel, NodeMessage, ShutdownType, WorkerBuilder,
};
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use ockam_core::compat::{boxed::Box, string::String, sync::Arc, vec::Vec};
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AddressSet, AllowAll, AsyncTryClone, Error, LocalMessage, Mailbox, Mailboxes, Message,
    Processor, Result, Route, TransportMessage, TransportType, Worker,
};
use ockam_core::{AccessControl, LocalInfo};

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

/// A special sender type that connects a type to an AsyncDrop handler
pub type AsyncDropSender = crate::tokio::sync::oneshot::Sender<Address>;

/// A special type of `Context` that has no worker relay and inherits
/// the parent `Context`'s access control
pub type DetachedContext = Context;

/// A special type of `Context` that has no worker relay and a custom
/// access control which is not inherited from its parent `Context.
pub type RepeaterContext = Context;

/// Context contains Node state and references to the runtime.
pub struct Context {
    mailboxes: Mailboxes,
    sender: SmallSender<NodeMessage>,
    rt: Arc<Runtime>,
    receiver: SmallReceiver<RelayMessage>,
    async_drop_sender: Option<AsyncDropSender>,
    mailbox_count: Arc<AtomicUsize>,
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Some(sender) = self.async_drop_sender.take() {
            trace!("De-allocated detached context {}", self.address());
            if let Err(e) = sender.send(self.address()) {
                warn!("Encountered error while dropping detached context: {}", e);
            }
        }
    }
}

#[ockam_core::async_trait]
impl AsyncTryClone for Context {
    async fn async_try_clone(&self) -> Result<Self> {
        self.new_detached(Address::random_local()).await
    }
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }

    /// Return mailbox_count clone
    pub(crate) fn mailbox_count(&self) -> Arc<AtomicUsize> {
        self.mailbox_count.clone()
    }

    /// Return a reference to sender
    pub(crate) fn sender(&self) -> &SmallSender<NodeMessage> {
        &self.sender
    }

    /// Wait for the next message from the mailbox
    pub(crate) async fn receiver_next(&mut self) -> Result<Option<RelayMessage>> {
        loop {
            let relay_msg = if let Some(msg) = self.receiver.recv().await.map(|msg| {
                trace!("{}: received new message!", self.address());

                // First we update the mailbox fill metrics
                self.mailbox_count.fetch_sub(1, Ordering::Acquire);

                msg
            }) {
                msg
            } else {
                return Ok(None);
            };

            if !self
                .mailboxes
                .is_authorized(&relay_msg.addr, &relay_msg.local_msg)
                .await?
            {
                warn!("Message for {} did not pass access control", relay_msg.addr);
                continue;
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
    ///
    /// `async_drop_sender` must be provided when creating a detached
    /// Context type (i.e. not backed by a worker relay).
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: SmallSender<NodeMessage>,
        mailboxes: Mailboxes,
        async_drop_sender: Option<AsyncDropSender>,
    ) -> (Self, SenderPair, SmallReceiver<CtrlSignal>) {
        let (mailbox_tx, receiver) = message_channel();
        let (ctrl_tx, ctrl_rx) = small_channel();
        (
            Self {
                rt,
                sender,
                mailboxes,
                receiver,
                async_drop_sender,
                mailbox_count: Arc::new(0.into()),
            },
            SenderPair {
                msgs: mailbox_tx,
                ctrl: ctrl_tx,
            },
            ctrl_rx,
        )
    }

    /// Return the primary address of the current worker
    pub fn address(&self) -> Address {
        self.mailboxes.main_address()
    }

    /// Return all addresses of the current worker
    pub fn aliases(&self) -> AddressSet {
        self.mailboxes.aliases()
    }

    /// Return a reference to the mailboxes of this context
    pub fn mailboxes(&self) -> &Mailboxes {
        &self.mailboxes
    }

    /// Utility function to sleep tasks from other crates
    #[doc(hidden)]
    pub async fn sleep(&self, dur: Duration) {
        tokio::time::sleep(dur).await;
    }

    /// Create a new detached `Context` that will apply the given
    /// [`AccessControl`] to any incoming messages it receives
    pub async fn new_repeater<AC>(&self, access_control: AC) -> Result<RepeaterContext>
    where
        AC: AccessControl,
    {
        let repeater_ctx = self
            .new_detached_impl(Mailboxes::main(
                Address::random_local(),
                Arc::new(access_control),
            ))
            .await?;
        Ok(repeater_ctx)
    }

    /// Create a new detached `Context` without spawning a full worker
    ///
    /// Note: this function is very low-level.  For most users
    /// [`start_worker()`](Self::start_worker) is the recommended to
    /// way to create a new worker context.
    pub async fn new_detached<S: Into<AddressSet>>(
        &self,
        address_set: S,
    ) -> Result<DetachedContext> {
        // Inherit access control
        let access_control = self.mailboxes.main_mailbox().access_control().clone();

        let mailboxes = Mailboxes::from_address_set(address_set.into(), access_control);

        self.new_detached_impl(mailboxes).await
    }

    async fn new_detached_impl(&self, mailboxes: Mailboxes) -> Result<DetachedContext> {
        // A detached Context exists without a worker relay, which
        // requires special shutdown handling.  To allow the Drop
        // handler to interact with the Node runtime, we use an
        // AsyncDrop handler.
        //
        // This handler is spawned and listens for an event from the
        // Drop handler, and then forwards a message to the Node
        // router.
        let (async_drop, drop_sender) = AsyncDrop::new(self.sender.clone());
        async_drop.spawn(&self.rt);

        // Create a new context and get access to the mailbox senders
        let addresses = mailboxes.addresses();
        let (ctx, sender, _) = Self::new(
            Arc::clone(&self.rt),
            self.sender.clone(),
            mailboxes,
            Some(drop_sender),
        );

        // Create a "detached relay" and register it with the router
        let (msg, mut rx) =
            NodeMessage::start_worker(addresses, sender, true, Arc::clone(&self.mailbox_count));
        self.sender
            .send(msg)
            .await
            .map_err(|e| Error::new(Origin::Node, Kind::Invalid, e))?;
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;

        Ok(ctx)
    }

    /// Start a new worker instance at the given address set
    ///
    /// A worker is an asynchronous piece of code that can send and
    /// receive messages of a specific type.  This type is encoded via
    /// the [`Worker`](ockam_core::Worker) trait.  If your code relies
    /// on a manual run-loop you may want to use
    /// [`start_processor()`](Self::start_processor) instead!
    ///
    /// Each address in the set must be unique and unused on the
    /// current node.  Workers must implement the Worker trait and be
    /// thread-safe.  Workers run asynchronously and will be scheduled
    /// independently of each other.  To wait for the initialisation
    /// of your worker to complete you can use
    /// [`wait_for()`](Self::wait_for).
    ///
    /// The worker will inherit its [`AccessControl`] from this
    /// context. Use [`WorkerBuilder`] to start a worker with custom
    /// access control.
    ///
    /// ```rust
    /// use ockam_core::{Result, Worker, worker};
    /// use ockam_node::Context;
    ///
    /// struct MyWorker;
    ///
    /// #[worker]
    /// impl Worker for MyWorker {
    ///     type Context = Context;
    ///     type Message = String;
    /// }
    ///
    /// async fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker("my-worker-address", MyWorker).await
    /// }
    /// ```
    pub async fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: Into<AddressSet>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        WorkerBuilder::with_inherited_access_control(self, address, worker)
            .start(self)
            .await?;
        Ok(())
    }

    /// Start a new processor instance at the given address set
    ///
    /// A processor is an asynchronous piece of code that runs a
    /// custom run loop, with access to a worker context to send and
    /// receive messages.  If your code is built around responding to
    /// message events, consider using
    /// [`start_worker()`](Self::start_processor) instead!
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

        let main_mailbox = Mailbox::new(addr, Arc::new(AllowAll)); // TODO FIXME
        let mailboxes = Mailboxes::new(main_mailbox, vec![]);

        let (ctx, senders, ctrl_rx) =
            Context::new(self.rt.clone(), self.sender.clone(), mailboxes, None);

        // Initialise the processor relay with the ctrl receiver
        ProcessorRelay::<P>::init(self.rt.as_ref(), processor, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_processor(address, senders);
        self.sender
            .send(msg)
            .await
            .map_err(|e| Error::new(Origin::Node, Kind::Invalid, e))?;

        // Wait for the actual return code
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }

    /// Shut down a local worker by its primary address
    pub async fn stop_worker<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Worker).await
    }

    /// Shut down a local processor by its address
    pub async fn stop_processor<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Processor).await
    }

    async fn stop_address(&self, addr: Address, t: AddressType) -> Result<()> {
        debug!("Shutting down {} {}", t.str(), addr);

        // Send the stop request
        let (req, mut rx) = match t {
            AddressType::Worker => NodeMessage::stop_worker(addr, false),
            AddressType::Processor => NodeMessage::stop_processor(addr),
        };
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;

        // Then check that address was properly shut down
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
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
            Err(e) => Err(Error::new(Origin::Node, Kind::Invalid, e)),
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
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;

        // Wait until we get the all-clear
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }

    /// Using a temporary new context, send a message and then receive a message
    ///
    /// This helper function uses [`new_detached`], [`send`], and
    /// [`receive`] internally. See their documentation for more
    /// details.
    ///
    /// [`new_detached`]: Self::new_detached
    /// [`send`]: Self::send
    /// [`receive`]: Self::receive
    pub async fn send_and_receive<R, M, N>(&self, route: R, msg: M) -> Result<N>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
        N: Message,
    {
        let mut child_ctx = self.new_detached(Address::random_local()).await?;
        child_ctx.send(route, msg).await?;
        Ok(child_ctx.receive::<N>().await?.take().body())
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
        if self.mailboxes.contains(&addr) {
            self.send_from_address(addr, msg, from.into()).await
        } else {
            Err(NodeError::NodeState(NodeReason::Unknown).internal())
        }
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// ```rust
    /// # use {ockam_node::Context, ockam_core::Result};
    /// # async fn test(ctx: &mut Context) -> Result<()> {
    /// use ockam_core::Message;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Message, Serialize, Deserialize)]
    /// struct MyMessage(String);
    ///
    /// impl MyMessage {
    ///     fn new(s: &str) -> Self {
    ///         Self(s.into())
    ///     }
    /// }
    ///
    /// ctx.send("my-test-worker", MyMessage::new("Hello you there :)")).await?;
    /// Ok(())
    /// # }
    /// ```
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.send_from_address(route.into(), msg, self.address())
            .await
    }

    /// Send a message to an address or via a fully-qualified route
    /// after attaching the given [`LocalInfo`] to the message.
    pub async fn send_with_local_info<R, M>(
        &self,
        route: R,
        msg: M,
        local_info: Vec<LocalInfo>,
    ) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.send_from_address_impl(route.into(), msg, self.address(), local_info)
            .await
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// This function additionally takes the sending address
    /// parameter, to specify which of a worker's (or processor's)
    /// addresses should be used.
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
        self.send_from_address_impl(route.into(), msg, sending_address, Vec::new())
            .await
    }

    async fn send_from_address_impl<M>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
        local_info: Vec<LocalInfo>,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
    {
        // Check if the sender address exists
        if !self.mailboxes.contains(&sending_address) {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        // First resolve the next hop in the route
        let (reply_tx, mut reply_rx) = small_channel();
        let next = route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_sender()?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().unwrap();
        let mut transport_msg = TransportMessage::v1(route.clone(), Route::new(), payload);
        transport_msg.return_route.modify().append(sending_address);

        // Pack transport message into a LocalMessage wrapper
        let local_msg = LocalMessage::new(transport_msg, local_info);

        // Pack local message into a RelayMessage wrapper
        let msg = RelayMessage::new(addr, local_msg, route, needs_wrapping);

        // Send the packed user message with associated route
        sender.send(msg).await.map_err(NodeError::from_send_err)?;

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
        // First resolve the next hop in the route
        let (reply_tx, mut reply_rx) = small_channel();
        let next = local_msg.transport().onward_route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_sender()?;

        // Pack the transport message into a relay message
        let onward = local_msg.transport().onward_route.clone();
        // let msg = RelayMessage::direct(addr, data, onward);
        let msg = RelayMessage::new(addr, local_msg, onward, needs_wrapping);

        // Forward the message
        sender.send(msg).await.map_err(NodeError::from_send_err)?;

        Ok(())
    }

    /// Block the current worker to wait for a typed message
    ///
    /// **Warning** this function will wait until its running ockam
    /// node is shut down.  A safer variant of this function is
    /// [`receive`](Self::receive) and
    /// [`receive_timeout`](Self::receive_timeout).
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

    /// Wait to receive a message up to a specified timeout
    ///
    /// See [`receive`](Self::receive) for more details.
    pub async fn receive_duration_timeout<M: Message>(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<Cancel<'_, M>> {
        let (msg, data, addr) = timeout(timeout_duration, async { self.next_from_mailbox().await })
            .await
            .map_err(|e| NodeError::Data.with_elapsed(e))??;
        Ok(Cancel::new(msg, data, addr, self))
    }

    /// Wait to receive a message up to a specified timeout
    ///
    /// See [`receive`](Self::receive) for more details.
    pub async fn receive_timeout<M: Message>(
        &mut self,
        timeout_secs: u64,
    ) -> Result<Cancel<'_, M>> {
        self.receive_duration_timeout(Duration::from_secs(timeout_secs))
            .await
    }

    /// Block the current worker to wait for a message satisfying a conditional
    ///
    /// Will return `Err` if the corresponding worker has been
    /// stopped, or the underlying node has shut down.  This operation
    /// has a [default timeout](DEFAULT_TIMEOUT).
    ///
    /// Internally this function uses [`receive`](Self::receive), so
    /// is subject to the same timeout.
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
        .map_err(|e| NodeError::Data.with_elapsed(e))??;

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
        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .is_ok()
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;

        reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_workers()
    }

    /// Register a router for a specific address type
    pub async fn register<A: Into<Address>>(&self, type_: TransportType, addr: A) -> Result<()> {
        self.register_impl(type_, addr.into()).await
    }

    /// Send a shutdown acknowledgement to the router
    pub(crate) async fn send_stop_ack(&self) -> Result<()> {
        self.sender
            .send(NodeMessage::StopAck(self.address()))
            .await
            .map_err(NodeError::from_send_err)?;
        Ok(())
    }

    async fn register_impl(&self, type_: TransportType, addr: Address) -> Result<()> {
        let (tx, mut rx) = small_channel();
        self.sender
            .send(NodeMessage::Router(type_, addr, tx))
            .await
            .map_err(NodeError::from_send_err)?;

        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
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
                .receiver_next()
                .await?
                .ok_or_else(|| NodeError::Data.not_found())?;
            let addr = msg.addr;
            let local_msg = msg.local_msg;

            // FIXME: make message parsing idempotent to avoid cloning
            match parser::message(&local_msg.transport().payload).ok() {
                Some(msg) => break Ok((msg, local_msg, addr)),
                None => {
                    // Requeue
                    self.forward(local_msg).await?;
                }
            }
        }
    }

    /// This function is called by Relay to indicate a worker is initialised
    pub(crate) async fn set_ready(&mut self) -> Result<()> {
        self.sender
            .send(NodeMessage::set_ready(self.address()))
            .await
            .map_err(NodeError::from_send_err)?;
        Ok(())
    }

    /// Wait for a particular address to become "ready"
    pub async fn wait_for<A: Into<Address>>(&mut self, addr: A) -> Result<()> {
        let (msg, mut reply) = NodeMessage::get_ready(addr.into());
        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;

        // This call blocks until the address has become ready or is
        // dropped by the router
        reply
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }
}
