// use crate::message::BaseMessage;

use crate::{
    router::{Router, SenderPair},
    NodeMessage,
};
use ockam_core::{Address, Result};

use crate::tokio::{runtime::Runtime, sync::mpsc::Sender};
use core::future::Future;
use ockam_core::compat::sync::Arc;

/// Ockam node and worker executor
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
    /// Create a new [`Executor`].
    pub fn new() -> Self {
        Executor::default()
    }

    /// Sender
    pub fn sender(&self) -> Sender<NodeMessage> {
        self.router.sender()
    }

    /// Runtime
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }

    /// Initialize the root application worker
    pub fn initialize_system<S: Into<Address>>(&mut self, address: S, senders: SenderPair) {
        trace!("Initializing node executor");
        self.router.init(address.into(), senders);
    }

    /// Execute a future
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
            .map_err(|_| crate::error::Error::ExecutorBodyJoinError)?;

        Ok(res)
    }

    #[cfg(not(feature = "std"))]
    /// Execute a future
    /// TODO @antoinevg - support @thomm join & merge with std version
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
