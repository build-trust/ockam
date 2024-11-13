use crate::Context;
use ockam_core::Result;

impl Context {
    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed.
    /// The default timeout for a safe shutdown is 1 second.  You can
    /// change this behaviour by calling
    /// [`Context::shutdown_node_with_timeout`](Context::shutdown_node_with_timeout) directly.
    pub async fn shutdown_node(&self) -> Result<()> {
        self.shutdown_node_with_timeout(1).await
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed
    /// or the desired timeout has been reached.
    pub async fn shutdown_node_with_timeout(&self, seconds: u8) -> Result<()> {
        let router = self.router()?;

        // Spawn a separate task, otherwise if this function is called from a worker, in can be
        // cancelled, as worker run loop itself is stopped as a result of this call
        let _handle = crate::spawn(async move { router.shutdown_graceful(seconds).await });

        #[cfg(feature = "std")]
        _handle.await.unwrap()?;

        // TODO: Would be cool to shutdown the Runtime here with a timeout in case router timed out,
        //  that would require more transparent ownership over the Runtime, since shutdown is
        //  consuming the value.

        Ok(())
    }
}
