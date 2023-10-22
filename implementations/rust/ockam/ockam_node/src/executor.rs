// use crate::message::BaseMessage;

use crate::channel_types::SmallSender;
use crate::{
    router::{Router, SenderPair},
    tokio::runtime::{Handle, Runtime},
    NodeMessage,
};
use core::future::Future;
use ockam_core::{Address, Result};

#[cfg(feature = "metrics")]
use crate::metrics::Metrics;

// This import is available on emebedded but we don't use the metrics
// collector, thus don't need it in scope.
#[cfg(feature = "metrics")]
use core::sync::atomic::{AtomicBool, Ordering};

use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Underlying Ockam node executor
///
/// This type is a small wrapper around an inner async runtime (`tokio` by
/// default) and the Ockam router. In most cases it is recommended you use the
/// `ockam::node` function annotation instead!
pub struct Executor {
    /// Reference to the runtime needed to spawn tasks
    rt: Runtime,
    /// Main worker and application router
    router: Router,
    /// Metrics collection endpoint
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
}

impl Executor {
    /// Create a new Ockam node [`Executor`] instance
    pub fn new(flow_controls: &FlowControls) -> Self {
        let rt = Runtime::new().unwrap();
        let router = Router::new(flow_controls);
        #[cfg(feature = "metrics")]
        let metrics = Metrics::new(&rt, router.get_metrics_readout());
        Self {
            rt,
            router,
            #[cfg(feature = "metrics")]
            metrics,
        }
    }

    /// Start the router asynchronously
    pub async fn start_router(&mut self) -> Result<()> {
        self.router.run().await
    }

    /// Get access to the internal message sender
    pub(crate) fn sender(&self) -> SmallSender<NodeMessage> {
        self.router.sender()
    }

    /// Get access to the underlying async runtime (by default `tokio`)
    pub(crate) fn runtime(&self) -> &Handle {
        self.rt.handle()
    }

    /// Initialize the root application worker
    pub(crate) fn initialize_system<S: Into<Address>>(&mut self, address: S, senders: SenderPair) {
        trace!("Initializing node executor");
        self.router.init(address.into(), senders);
    }

    /// Initialise and run the Ockam node executor context
    ///
    /// In this background this launches async execution of the Ockam
    /// router, while blocking execution on the provided future.
    ///
    /// Any errors encountered by the router or provided application
    /// code will be returned from this function.
    #[cfg(feature = "std")]
    pub fn execute<F, T>(&mut self, future: F) -> Result<F::Output>
    where
        F: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        // Spawn the metrics collector first
        #[cfg(feature = "metrics")]
        let alive = Arc::new(AtomicBool::from(true));
        #[cfg(feature = "metrics")]
        self.rt.spawn(self.metrics.clone().run(alive.clone()));

        // Spawn user code second
        let sender = self.sender();
        let future = Executor::wrapper(sender, future);
        let join_body = self.rt.spawn(future);

        // Then block on the execution of the router
        self.rt.block_on(self.router.run())?;

        // Shut down metrics collector
        #[cfg(feature = "metrics")]
        alive.fetch_or(true, Ordering::Acquire);

        // Last join user code
        let res = self
            .rt
            .block_on(join_body)
            .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))?;

        Ok(res)
    }

    /// Wrapper around the user provided future that will shut down the node on error
    #[cfg(feature = "std")]
    async fn wrapper<F, T>(sender: SmallSender<NodeMessage>, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>> + Send + 'static,
    {
        match future.await {
            Ok(val) => Ok(val),
            Err(e) => {
                // We earlier sent the AbortNode message to the router here.
                // It failed because the router state was not set to `Stopping`
                // But sending Gracefull shutdown message works because, it internally does that.
                //
                // I think way AbortNode is implemented right now, it is more of an
                // internal/private message not meant to be directly used, without changing the
                // router state.
                let (req, mut rx) = NodeMessage::stop_node(crate::ShutdownType::Graceful(1));
                let _ = sender.send(req).await;
                let _ = rx.recv().await;
                Err(e)
            }
        }
    }

    /// Execute a future and block until a result is returned
    #[cfg(feature = "std")]
    pub fn execute_future<F>(future: F) -> Result<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let rt = Runtime::new().unwrap();
        let join_body = rt.spawn(future);
        rt.block_on(join_body)
            .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))
    }

    #[cfg(not(feature = "std"))]
    /// Initialise and run the Ockam node executor context
    ///
    /// In this background this launches async execution of the Ockam
    /// router, while blocking execution on the provided future.
    ///
    /// Any errors encountered by the router or provided application
    /// code will be returned from this function.
    // TODO @antoinevg - support @thomm join & merge with std version
    pub fn execute<F>(&mut self, future: F) -> Result<()>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let _join = self.rt.spawn(future);

        // Block this task executing the primary message router,
        // returning any critical failures that it encounters.
        let future = self.router.run();
        crate::tokio::runtime::execute(&self.rt, async move { future.await.unwrap() });
        Ok(())
    }
}
