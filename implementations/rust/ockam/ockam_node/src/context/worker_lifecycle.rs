use crate::{Context, NodeError, NodeMessage, NodeReason};
use crate::{ProcessorBuilder, WorkerBuilder};
use ockam_core::{
    Address, IncomingAccessControl, OutgoingAccessControl, Processor, Result, Worker,
};

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

impl Context {
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
    /// async fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker("my-worker-address", MyWorker).await
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
    pub async fn start_worker<W>(&self, address: impl Into<Address>, worker: W) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        WorkerBuilder::new(worker)
            .with_address(address)
            .start(self)
            .await?;

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
    /// independently of each other.  To wait for the initialisation
    /// of your worker to complete you can use
    /// [`wait_for()`](Self::wait_for).
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
    /// async fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker_with_access_control("my-worker-address", MyWorker, AllowAll, AllowAll).await
    /// }
    /// ```
    pub async fn start_worker_with_access_control<W>(
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
            .start(self)
            .await?;

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
    pub async fn start_processor<P>(&self, address: impl Into<Address>, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        ProcessorBuilder::new(processor)
            .with_address(address.into())
            .start(self)
            .await?;

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
    pub async fn start_processor_with_access_control<P>(
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
            .start(self)
            .await?;

        Ok(())
    }

    /// Shut down a local worker by its primary address
    ///
    /// Approximate flow of stopping a worker:
    ///
    /// 1. StopWorker message -> Router
    /// 2. Get AddressRecord
    /// 3. Drop sender
    /// 4. WorkerRelay calls Worker::shutdown
    /// 5. StopAck message -> Router (from main_address)
    /// 6. router.map.free_address(main_address) is called (given Router state is running):
    ///     remote main_address from router.map.stopping (it's not their anyway, unless in was a cluster and node was shutting down)
    ///     Remove AddressRecord from router.map.address_records_map (return error if not found)
    ///     Remove all alias in router.map.alias_map
    ///     Remote all meta from router.map.address_metadata
    pub async fn stop_worker<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Worker).await
    }

    /// Shut down a local processor by its address
    ///
    /// Approximate flow of stopping a processor:
    ///
    /// 1. StopProcessor message -> Router
    /// 2. Get AddressRecord
    /// 3. Call AddressRecord::stop:
    ///     Send CtrlSignal::InterruptStop to AddressRecord::ctrl_tx
    ///     Set AddressRecord::state = AddressState::Stopping
    /// 4. ProcessorRelay calls Processor::shutdown
    /// 5. StopAck message -> Router (from main_address)
    /// 6. router.map.free_address(main_address) is called (given Router state is running):
    ///     remote main_address from router.map.stopping (it's not their anyways unless in was a cluster and node was shutting down)
    ///     Remove AddressRecord from router.map.address_records_map (return error if not found)
    ///     Remove all alias in router.map.alias_map
    ///     Remote all meta from router.map.address_metadata
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
}
