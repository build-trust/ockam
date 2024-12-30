// TODO: Use either stop or shutdown everywhere

/// Specify the type of node shutdown
///
/// For most users `ShutdownType::Graceful()` is recommended.  The
/// `Default` implementation uses a 1 second timeout.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ShutdownType {
    /// Execute a graceful shutdown given a maximum timeout
    ///
    /// The following steps will be taken by the internal router
    /// during graceful shutdown procedure:
    ///
    /// * Signal clusterless workers to stop
    /// * Wait for shutdown ACK hooks from worker set
    /// * Signal worker clusters in reverse-creation order to stop
    /// * Wait for shutdown ACK hooks from each cluster before moving onto the
    ///   next
    /// * All shutdown-signaled workers may process their entire mailbox,
    ///   while not allowing new messages to be queued
    ///
    /// Graceful shutdown procedure will be pre-maturely terminated
    /// when reaching the timeout (failover into `Immediate`
    /// strategy).  **A given timeout of `0` will wait forever!**
    Graceful(u8),
    /// Immediately shutdown workers and run shutdown hooks
    ///
    /// This strategy can lead to data loss:
    ///
    /// * Unhandled mailbox messages will be dropped
    /// * Shutdown hooks may not be able to send messages
    ///
    /// This strategy is not recommended for general use, but will be
    /// selected as a failover, if the `Graceful` strategy reaches its
    /// timeout limit.
    Immediate,
}

impl Default for ShutdownType {
    fn default() -> Self {
        Self::Graceful(1)
    }
}
