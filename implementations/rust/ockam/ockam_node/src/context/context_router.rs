use crate::channel_types::OneshotReceiver;
use crate::relay::CtrlSignal;
use crate::router::{Router, SenderPair};
use crate::tokio::runtime::Handle;
use crate::{Context, ContextMode, MessageSendReceiveOptions, ProcessorBuilder, WorkerBuilder};
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Weak;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{
    Address, AddressMetadata, Error, IncomingAccessControl, Mailboxes, Message,
    OutgoingAccessControl, Processor, Result, Route, Routed, TransportType, Worker,
};
use ockam_transport_core::Transport;

/// [`Context`] counterpart that allows starting/stopping workers and doesn't have its own mailbox
#[derive(Clone)]
pub struct ContextRouter {
    pub(super) router: Weak<Router>,
    pub(super) mailbox_count: Arc<AtomicUsize>,
    pub(super) flow_controls: FlowControls,
    /// List of transports used to resolve external addresses to local workers in routes
    pub(super) transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
    pub(super) runtime_handle: Handle,
    #[cfg(feature = "std")]
    pub(super) tracing_context: OpenTelemetryContext,
}

impl ContextRouter {
    /// Start a new worker instance at the given address. Default AccessControl is AllowAll
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
    /// fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker("my-worker-address", MyWorker)
    /// }
    /// ```
    ///
    /// Approximate flow of starting a worker:
    ///
    /// 1. StartWorker message -> Router
    /// 2. First address is considered a primary_addr (main_addr)
    /// 3. Check if router.map.address_records_map already has primary_addr
    /// 4. AddressRecord is created and inserted in router.map
    /// 5. Iterate over metadata:
    ///     Check if it belongs to that record
    ///     Set is_terminal true in router.map.address_metadata_map (if address is terminal)
    ///     Insert attributes one by one
    /// 6. For each address we insert pair (Address, primary_addr) into router.map.alias_map, including (primary_addr, primary_addr itself)
    /// 7. WorkerRelay is spawned as a tokio task:
    ///     WorkerRelay calls initialize
    ///     WorkerRelay calls Worker::handle_message for each message until either
    ///         stop signal is received (CtrlSignal::InterruptStop to AddressRecord::ctrl_tx)
    ///         there are no messages coming to that receiver (the sender side is dropped)
    pub fn start_worker<W>(&self, address: impl Into<Address>, worker: W) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        WorkerBuilder::new(worker)
            .with_address(address)
            .start_using_router_context(self)?;

        Ok(())
    }

    /// Start a new worker instance at the given address
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
    /// independently of each other.
    ///
    /// ```rust
    /// use ockam_core::{AllowAll, Result, Worker, worker};
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
    ///  fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker_with_access_control("my-worker-address", MyWorker, AllowAll, AllowAll)
    /// }
    /// ```
    pub fn start_worker_with_access_control<W>(
        &self,
        address: impl Into<Address>,
        worker: W,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control(incoming)
            .with_outgoing_access_control(outgoing)
            .start_using_router_context(self)?;

        Ok(())
    }

    /// Start a new processor instance at the given address. Default AccessControl is DenyAll
    ///
    /// A processor is an asynchronous piece of code that runs a
    /// custom run loop, with access to a worker context to send and
    /// receive messages.  If your code is built around responding to
    /// message events, consider using
    /// [`start_worker()`](Self::start_worker) instead!
    ///
    /// Approximate flow of starting a processor:
    ///
    /// 1. StartProcessor message -> Router
    /// 2. First address is considered a primary_addr (main_addr)
    /// 3. Check if router.map.address_records_map already has primary_addr
    /// 4. AddressRecord is created and inserted in router.map
    /// 5. Iterate over metadata:
    ///     Check if it belongs to that record
    ///     Set is_terminal true in router.map.address_metadata_map (if address is terminal)
    ///     Insert attributes one by one
    /// 6. For each address we insert pair (Address, primary_addr) into router.map.alias_map, including (primary_addr, primary_addr itself)
    /// 7. ProcessorRelay is spawned as a tokio task:
    ///     ProcessorRelay calls Processor::initialize
    ///     ProcessorRelay calls Processor::process until either false is returned or stop signal is received (CtrlSignal::InterruptStop to AddressRecord::ctrl_tx)
    pub fn start_processor<P>(&self, address: impl Into<Address>, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        ProcessorBuilder::new(processor)
            .with_address(address.into())
            .start_using_router_context(self)?;

        Ok(())
    }

    /// Start a new processor instance at the given address
    ///
    /// A processor is an asynchronous piece of code that runs a
    /// custom run loop, with access to a worker context to send and
    /// receive messages.  If your code is built around responding to
    /// message events, consider using
    /// [`start_worker()`](Self::start_worker) instead!
    ///
    pub fn start_processor_with_access_control<P>(
        &self,
        address: impl Into<Address>,
        processor: P,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        ProcessorBuilder::new(processor)
            .with_address(address)
            .with_incoming_access_control(incoming)
            .with_outgoing_access_control(outgoing)
            .start_using_router_context(self)?;

        Ok(())
    }

    /// Stop a Worker or a Processor running on given Address
    pub fn stop_address(&self, address: &Address) -> Result<()> {
        self.router()?.stop_address(address, false)
    }

    /// Reference to the Router
    pub(crate) fn router(&self) -> Result<Arc<Router>> {
        self.router
            .upgrade()
            .ok_or_else(|| Error::new(Origin::Node, Kind::Shutdown, "Failed to upgrade router"))
    }

    /// Return runtime clone
    pub fn runtime(&self) -> &Handle {
        &self.runtime_handle
    }

    /// Return mailbox_count clone
    pub(crate) fn mailbox_count(&self) -> Arc<AtomicUsize> {
        self.mailbox_count.clone()
    }

    pub(crate) fn new_with_mailboxes(
        &self,
        mailboxes: Mailboxes,
        mode: ContextMode,
    ) -> (Context, SenderPair, OneshotReceiver<CtrlSignal>) {
        Context::new(
            self.runtime().clone(),
            self.router.clone(),
            mailboxes,
            mode,
            self.transports.clone(),
            &self.flow_controls,
            #[cfg(feature = "std")]
            OpenTelemetryContext::current(),
        )
    }

    /// Using a temporary new context, send a message and then receive a message
    /// with default timeout and no flow control
    ///
    /// This helper function uses [`new_detached`], [`send`], and
    /// [`receive`] internally. See their documentation for more
    /// details.
    ///
    /// [`new_detached`]: Self::new_detached
    /// [`send`]: Self::send
    /// [`receive`]: Self::receive
    pub async fn send_and_receive<T, R>(&self, route: impl Into<Route>, msg: T) -> Result<R>
    where
        T: Message,
        R: Message,
    {
        self.send_and_receive_extended::<T, R>(route, msg, MessageSendReceiveOptions::new())
            .await?
            .into_body()
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
    pub async fn send_and_receive_extended<T, R>(
        &self,
        route: impl Into<Route>,
        msg: T,
        options: MessageSendReceiveOptions,
    ) -> Result<Routed<R>>
    where
        T: Message,
        R: Message,
    {
        Context::send_and_receive_extended_impl(
            self.runtime().clone(),
            self.router()?,
            self.transports.clone(),
            &self.flow_controls,
            self.mailbox_count(),
            route.into(),
            msg,
            options,
            #[cfg(feature = "std")]
            self.tracing_context.clone(),
        )
        .await
    }

    /// Create a new detached `Context` without spawning a full worker
    ///
    /// Note: this function is very low-level.  For most users
    /// [`start_worker()`](Self::start_worker) is the recommended way
    /// to create a new worker context.
    ///
    /// Approximate flow of starting a detached address:
    ///
    /// 1. Create and Spawn AsyncDrop::run
    /// 2. StartWorker message -> Router
    /// 3. First address is considered a primary_addr (main_addr)
    /// 4. Check if router.map.address_records_map already has primary_addr
    /// 5. AddressRecord is created and inserted in router.map
    /// 6. Iterate over metadata:
    ///     Check if it belongs to that record
    ///     Set is_terminal true in router.map.address_metadata_map (if address is terminal)
    ///     Insert attributes one by one
    /// 7. For each address we insert pair (Address, primary_addr) into router.map.alias_map, including (primary_addr, primary_addr itself)
    ///
    /// Approximate flow of stopping a detached address:
    ///
    /// 1. Context::Drop is called when Context is dropped by rust runtime (according to RAII principle)
    /// 2. async_drop_sender is used to send the Context address
    /// 3. AsyncDrop sends StopWorker message -> Router
    /// 4. Get AddressRecord
    /// 5. router.map.free_address(main_address) is called (given Router state is running):
    ///     remote main_address from router.map.stopping (it's not their anyway, unless in was a cluster and node was shutting down)
    ///     Remove AddressRecord from router.map.address_records_map (return error if not found)
    ///     Remove all alias in router.map.alias_map
    ///     Remote all meta from router.map.address_metadata
    pub fn new_detached(
        &self,
        address: impl Into<Address>,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<Context> {
        let mailboxes = Mailboxes::primary(address.into(), Arc::new(incoming), Arc::new(outgoing));
        self.new_detached_with_mailboxes(mailboxes)
    }

    /// Create a new [`Context`] instance with dedicated Mailbox(es)
    pub fn new_detached_with_mailboxes(&self, mailboxes: Mailboxes) -> Result<Context> {
        Context::new_detached_with_mailboxes_impl(
            self.runtime().clone(),
            self.router()?,
            self.transports.clone(),
            &self.flow_controls,
            mailboxes,
            self.mailbox_count(),
        )
    }
}

impl ContextRouter {
    /// Return a list of all available worker addresses on a node
    pub fn list_workers(&self) -> Result<Vec<Address>> {
        Ok(self.router()?.list_workers())
    }

    /// Return true if a worker is already registered at this address
    pub fn is_worker_registered_at(&self, address: &Address) -> Result<bool> {
        Ok(self.router()?.is_worker_registered_at(address))
    }

    /// Finds the terminal address of a route, if present
    pub fn find_terminal_address<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Address>,
    ) -> Result<Option<(&'a Address, AddressMetadata)>> {
        Ok(self.router()?.find_terminal_address(addresses))
    }

    /// Read metadata for the provided address
    pub fn get_metadata(&self, address: &Address) -> Result<Option<AddressMetadata>> {
        Ok(self.router()?.get_address_metadata(address))
    }
}

impl Context {
    /// Get cloneable [`Context`] variant that allows shared access to sending capabilities from
    /// a specified address. Doesn't own any mailbox, therefore original [`Context`] must be alive
    /// to be able to send the message.
    pub fn get_router_context(&self) -> ContextRouter {
        ContextRouter {
            router: self.router_weak(),
            mailbox_count: self.mailbox_count(),
            flow_controls: self.flow_controls().clone(),
            transports: self.transports.clone(),
            runtime_handle: self.runtime_handle.clone(),
            #[cfg(feature = "std")]
            tracing_context: OpenTelemetryContext::current(),
        }
    }
}

impl From<Context> for ContextRouter {
    fn from(value: Context) -> Self {
        value.get_router_context()
    }
}
