use crate::{Context, Worker, WorkerBuilder};
use ockam_core::{Address, IncomingAccessControl, OutgoingAccessControl, Result};

impl Context {
    /// Start a new worker instance at the given address. Default AccessControl is AllowAll
    ///
    /// A worker is an asynchronous piece of code that can send and
    /// receive messages of a specific type.  This type is encoded via
    /// the [`Worker`](ockam_core::Worker) trait.  If your code relies
    /// on a manual run-loop you may want to use
    /// [`start_worker()`](Self::start_worker) instead!
    ///
    /// Each address in the set must be unique and unused on the
    /// current node.  Workers must implement the Worker trait and be
    /// thread-safe.  Workers run asynchronously and will be scheduled
    /// independently of each other.  To wait for the initialisation
    /// of your worker to complete you can use
    /// [`wait_for()`](Self::wait_for).
    ///
    /// ```rust
    /// use ockam_core::{Result, worker};
    /// use ockam_node::{Worker, Context};
    ///
    /// struct MyWorker;
    ///
    /// #[worker]
    /// impl Worker for MyWorker {
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
    pub fn start_worker<W: Worker>(&self, address: impl Into<Address>, worker: W) -> Result<()> {
        WorkerBuilder::new(worker)
            .with_address(address)
            .start(self)?;

        Ok(())
    }

    /// Start a new worker instance at the given address
    ///
    /// A worker is an asynchronous piece of code that can send and
    /// receive messages of a specific type.  This type is encoded via
    /// the [`Worker`](ockam_core::Worker) trait.  If your code relies
    /// on a manual run-loop you may want to use
    /// [`start_worker()`](Self::start_worker) instead!
    ///
    /// Each address in the set must be unique and unused on the
    /// current node.  Workers must implement the Worker trait and be
    /// thread-safe.  Workers run asynchronously and will be scheduled
    /// independently of each other.
    ///
    /// ```rust
    /// use ockam_core::{AllowAll, Result, worker};
    /// use ockam_node::{Context, Worker};
    ///
    /// struct MyWorker;
    ///
    /// #[worker]
    /// impl Worker for MyWorker {
    ///     type Message = String;
    /// }
    ///
    ///  fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker_with_access_control("my-worker-address", MyWorker, AllowAll, AllowAll)
    /// }
    /// ```
    pub fn start_worker_with_access_control<W: Worker>(
        &self,
        address: impl Into<Address>,
        worker: W,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()> {
        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control(incoming)
            .with_outgoing_access_control(outgoing)
            .start(self)?;

        Ok(())
    }

    /// Stop a Worker or a Processor running on given Address
    pub fn stop_address(&self, address: &Address) -> Result<()> {
        self.router()?.stop_address(address, false)
    }
}
