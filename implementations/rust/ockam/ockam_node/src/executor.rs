// use crate::message::BaseMessage;

use crate::{relay::RelayMessage, router::Router, NodeMessage};
use ockam_core::{Address, Result};

use std::{future::Future, sync::Arc};
use tokio::{runtime::Runtime, sync::mpsc::Sender};

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

    pub(crate) fn sender(&self) -> Sender<NodeMessage> {
        self.router.sender()
    }

    pub(crate) fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }

    /// Initialize the root application worker
    pub fn initialize_system<S: Into<Address>>(
        &mut self,
        address: S,
        mailbox: Sender<RelayMessage>,
    ) {
        trace!("Initializing node executor");
        self.router.init(address.into(), mailbox);
    }

    pub fn execute<F>(&mut self, future: F) -> Result<()>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let rt = Arc::clone(&self.rt);
        let _join = rt.spawn(future);

        // Block this task executing the primary message router,
        // returning any critical failures that it encounters.
        rt.block_on(self.router.run())
    }
}
