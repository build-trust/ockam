use core::sync::atomic::{AtomicUsize, Ordering};

/// A simple monotonic ID generator
pub(crate) struct Monotonic {
    inner: AtomicUsize,
}

impl Monotonic {
    /// Create a new monotonic ID counter from 0
    pub(crate) fn new() -> Self {
        Self::from(0)
    }

    /// Create a new monotonic ID counter from a starting point
    ///
    /// This is useful when dealing with 1-indexed systems
    pub(crate) fn from(u: usize) -> Self {
        Monotonic { inner: u.into() }
    }

    pub(crate) fn next(&self) -> usize {
        self.inner.fetch_add(1, Ordering::Relaxed)
    }
}
