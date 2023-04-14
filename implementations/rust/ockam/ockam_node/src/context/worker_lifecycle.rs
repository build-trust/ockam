use crate::{Context, NodeError, NodeMessage, NodeReason};
use crate::{ProcessorBuilder, WorkerBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, IncomingAccessControl, Mailboxes, Message, OutgoingAccessControl, Processor,
    RelayMessage, Result, Worker,
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
    ///     ctx.start_worker("my-worker-address", MyWorker, AllowAll, AllowAll).await
    /// }
    /// ```
    pub async fn start_worker<NM, NW>(
        &self,
        address: impl Into<Address>,
        worker: NW,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        WorkerBuilder::with_mailboxes(
            Mailboxes::main(address, Arc::new(incoming), Arc::new(outgoing)),
            worker,
        )
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
    pub async fn start_processor<P>(
        &self,
        address: impl Into<Address>,
        processor: P,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        ProcessorBuilder::with_mailboxes(
            Mailboxes::main(address.into(), Arc::new(incoming), Arc::new(outgoing)),
            processor,
        )
        .start(self)
        .await?;

        Ok(())
    }

    /// Shut down a local worker by its primary address
    pub async fn stop_worker<A: Into<Address>>(&self, addr: A) -> Result<()> {
        self.stop_address(addr.into(), AddressType::Worker).await
    }

    /// Stop a worker and then extract all messages left in the queue
    pub async fn deconstruct<A: Into<Address>>(&mut self, addr: A) -> Result<Vec<RelayMessage>> {
        self.stop_address(addr.into(), AddressType::Worker).await?;

        let mut relay_messages = vec![];
        while let Some(next) = self.receiver_next().await? {
            relay_messages.push(next);
        }

        Ok(relay_messages)
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
}
