use crate::{
    async_trait,
    compat::{boxed::Box, string::String},
    Address, AddressSet, Cancel, LocalMessage, Message, Processor, Result, Route, Worker,
};
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;

/// The API and other resources available for the worker during message
/// processing.
#[async_trait]
pub trait NodeContext: Send + Sync + Sized + 'static {
    /// Return the primary worker address
    fn address(&self) -> Address;
    /// Return all addresses of this worker
    fn aliases(&self) -> AddressSet;

    /// Returns the default timeout for recieves, normally 30 seconds.
    fn default_timeout(&self) -> Duration {
        core::time::Duration::from_secs(30)
    }

    /// Create a new context without spawning a full worker
    async fn new_context(&self, addr: Address) -> Result<Self>;

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed.
    /// The default timeout for a safe shutdown is 1 second.  You can
    /// change this behaviour by calling
    /// [`NodeContext::stop_timeout`](NodeContext::stop_timeout) directly.
    async fn stop(&mut self) -> Result<()>;

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed
    /// or the desired timeout has been reached.
    async fn stop_timeout(&mut self, d: Duration) -> Result<()>;

    /// Send a message via a fully qualified route using specific Worker address
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`RouteBuilder`]: crate::RouteBuilder
    async fn send_from_address<M: Message>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
    ) -> Result<()>;

    /// Send a message via a fully qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`RouteBuilder`]: crate::RouteBuilder
    fn send<'a, R, M: Message>(
        &'a self,
        route: R,
        msg: M,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where
        Self: Sized,
        R: Into<Route> + 'static,
    {
        self.send_from_address(route.into(), msg, self.address())
    }

    /// Block the current worker to wait for a message satisfying a conditional
    ///
    /// Will return `Err` if the corresponding worker has been
    /// stopped, or the underlying node has shut down.  This operation
    /// has a [default timeout](Self::default_timeout).
    ///
    /// Internally this function calls `receive` and `.cancel()` in a
    /// loop until a matching message is found.
    async fn receive_match<'a, M, F>(&'a mut self, check: F) -> Result<Cancel<'a, Self, M>>
    where
        M: Message,
        F: Send + Sync + Fn(&M) -> bool;

    /// Receive a message without a timeout
    async fn receive_block<'a, M: Message>(&mut self) -> Result<Cancel<'_, Self, M>>;

    /// Block the current worker to wait for a typed message
    ///
    /// This function may return a `Err(FailedLoadData)` if the
    /// underlying worker was shut down, or `Err(Timeout)` if the call
    /// was waiting for longer than the `default timeout`.  Use
    /// [`receive_timeout`](NodeContext::receive_timeout) to adjust the
    /// timeout period.
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    async fn receive<'a, M: Message>(&'a mut self) -> Result<Cancel<'a, Self, M>> {
        self.receive_timeout(self.default_timeout()).await
    }

    /// Block to wait for a typed message, with explicit timeout
    async fn receive_timeout<'a, M: Message>(
        &'a mut self,
        timeout: Duration,
    ) -> Result<Cancel<'a, Self, M>>;

    /// Start a new worker handle at [`Address`]
    async fn start_worker<W, M>(&self, address: AddressSet, worker: W) -> Result<()>
    where
        M: Message,
        W: Worker<Self, Message = M>;

    /// Start a new processor at [`Address`]
    async fn start_processor<P>(&self, address: Address, processor: P) -> Result<()>
    where
        P: Processor<Self>;

    /// Shut down a worker by its primary address
    async fn stop_worker(&self, addr: Address) -> Result<()>;
    /// Shut down a processor by its address
    async fn stop_processor(&self, addr: Address) -> Result<()>;

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`NodeContext::send`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`NodeContext::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`NodeContext::send`]: Self::send
    /// [`TransportMessage`]: crate::TransportMessage
    async fn forward(&self, local_msg: LocalMessage) -> Result<()>;

    /// Register a router for a specific address type
    async fn register(&self, ty: u8, addr: Address) -> Result<()>;

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
    async fn set_cluster(&self, label: String) -> Result<()>;

    /// Spawn a future on this node's runtime.
    // TODO: this belongs on some async runtime abstraction.
    fn spawn_detached<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;

    // TODO: this belongs on some async runtime abstraction.
    #[doc(hidden)]
    async fn sleep(&self, d: Duration);
}
