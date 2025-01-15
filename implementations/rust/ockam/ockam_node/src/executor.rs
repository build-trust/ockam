use crate::{router::Router, tokio::runtime::Runtime};
use core::future::Future;
use ockam_core::{
    compat::sync::{Arc, Weak},
    Result,
};

#[cfg(feature = "metrics")]
use crate::metrics::Metrics;

// This import is available on emebedded but we don't use the metrics
// collector, thus don't need it in scope.
#[cfg(feature = "metrics")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "std")]
use opentelemetry::trace::FutureExt;

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
    runtime: Arc<Runtime>,
    /// Application router
    router: Arc<Router>,
    /// Metrics collection endpoint
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
}

impl Executor {
    /// Create a new Ockam node [`Executor`] instance
    pub fn new(runtime: Arc<Runtime>, flow_controls: &FlowControls) -> Self {
        let router = Arc::new(Router::new(flow_controls));
        #[cfg(feature = "metrics")]
        let metrics = Metrics::new(runtime.handle().clone(), router.get_metrics_readout());
        Self {
            runtime,
            router,
            #[cfg(feature = "metrics")]
            metrics,
        }
    }

    /// Get access to the Router
    pub(crate) fn router(&self) -> Weak<Router> {
        Arc::downgrade(&self.router)
    }

    /// Return the runtime
    pub fn get_runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }

    /// Initialise and run the Ockam node executor context
    ///
    /// Any errors encountered by the router or provided application
    /// code will be returned from this function.
    #[cfg(feature = "std")]
    pub fn execute<F, T, E>(&mut self, future: F) -> Result<F::Output>
    where
        F: Future<Output = core::result::Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        // Spawn the metrics collector first
        #[cfg(feature = "metrics")]
        let alive = Arc::new(AtomicBool::from(true));
        #[cfg(feature = "metrics")]
        self.metrics.clone().spawn(alive.clone());

        // Spawn user code second
        let future = Executor::wrapper(self.router.clone(), future);
        let join_body = self.runtime.spawn(future.with_current_context());

        // Shut down metrics collector
        #[cfg(feature = "metrics")]
        alive.fetch_or(true, Ordering::Acquire);

        // Last join user code
        let res = self
            .runtime
            .block_on(join_body)
            .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))?;

        // TODO: Shutdown Runtime if we exclusively own it. Which should be always except when we
        //  run multiple nodes inside the same process

        Ok(res)
    }

    /// Wrapper around the user provided future that will shut down the node on error
    #[cfg(feature = "std")]
    async fn wrapper<F, T, E>(router: Arc<Router>, future: F) -> core::result::Result<T, E>
    where
        F: Future<Output = core::result::Result<T, E>> + Send + 'static,
    {
        match future.await {
            Ok(val) => {
                debug!("Wait for router termination...");
                router.wait_termination().await;
                debug!("Router terminated successfully!...");
                Ok(val)
            }
            Err(e) => {
                if let Err(error) = router.shutdown_graceful(1).await {
                    error!("Failed to stop gracefully: {}", error);
                }
                Err(e)
            }
        }
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
        let _join = self.runtime.spawn(future);
        let router = self.router.clone();

        // Block this task executing the primary message router,
        // returning any critical failures that it encounters.
        crate::tokio::runtime::execute(&self.runtime, async move {
            router.wait_termination().await;
        });
        Ok(())
    }
}
