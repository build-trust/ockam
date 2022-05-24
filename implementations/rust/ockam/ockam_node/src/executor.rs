// use crate::message::BaseMessage;

use crate::tokio::runtime::Runtime;
use crate::{
    router::{Router, SenderPair},
    NodeMessage,
};
use core::future::Future;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result};

use crate::channel_types::SmallSender;
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
    rt: Arc<Runtime>,
    /// Main worker and application router
    router: Router,
}

impl Default for Executor {
    fn default() -> Self {
        let rt = Arc::new(Runtime::new().unwrap());
        let router = Router::new();
        Self { rt, router }
    }
}

impl Executor {
    /// Create a new Ockam node [`Executor`] instance
    pub fn new() -> Self {
        Executor::default()
    }

    /// Get access to the internal message sender
    pub(crate) fn sender(&self) -> SmallSender<NodeMessage> {
        self.router.sender()
    }

    /// Get access to the underlying async runtime (by default `tokio`)
    pub(crate) fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
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
    pub fn execute<F>(&mut self, future: F) -> Result<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let rt = Arc::clone(&self.rt);
        let join_body = rt.spawn(future);

        crate::block_future(&rt, async move { self.router.run().await })?;

        let res = crate::block_future(&rt, async move { join_body.await })
            .map_err(|e| Error::new(Origin::Executor, Kind::Unknown, e))?;

        Ok(res)
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
        let rt = Arc::clone(&self.rt);
        let _join = rt.spawn(future);

        // Block this task executing the primary message router,
        // returning any critical failures that it encounters.
        crate::tokio::runtime::execute(&rt, async move { self.router.run().await.unwrap() });
        Ok(())
    }
}
