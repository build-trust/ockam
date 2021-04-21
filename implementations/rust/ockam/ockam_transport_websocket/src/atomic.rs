//! Small utilities for working with atomic bools

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Type alias for Arc<AtomicBool>
pub type ArcBool = Arc<AtomicBool>;

/// Create a new ArcBool
pub fn new(b: bool) -> ArcBool {
    Arc::new(AtomicBool::new(b))
}

/// Stop the ArcBool
pub fn stop(b: &ArcBool) {
    b.fetch_and(false, Ordering::Relaxed);
}

/// Perform a relaxed ordering check
pub fn check(b: &ArcBool) -> bool {
    b.load(Ordering::Relaxed)
}
