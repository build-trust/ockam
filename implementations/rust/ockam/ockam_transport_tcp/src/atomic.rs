//! Small utilities for working with atomic bools

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// TODO: replace with a Future which can be polled, so that
// TcpRecvWorker and ListeningWorker can both select on the run
// future, and the tokio read futures.
pub(crate) type ArcBool = Arc<AtomicBool>;

/// Create a new ArcBool
pub(crate) fn new(b: bool) -> ArcBool {
    Arc::new(AtomicBool::new(b))
}

/// Stop the ArcBool
pub(crate) fn stop(b: &ArcBool) {
    b.fetch_and(false, Ordering::Relaxed);
}

/// Perform a relaxed ordering check
pub(crate) fn check(b: &ArcBool) -> bool {
    b.load(Ordering::Relaxed)
}
