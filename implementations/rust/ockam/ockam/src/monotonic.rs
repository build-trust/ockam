#![allow(unused)]
use std::sync::atomic::{AtomicUsize, Ordering};

/// A simple monotonic ID generator
pub(crate) struct Monotonic {
    inner: AtomicUsize,
}

impl Monotonic {
    pub(crate) fn new() -> Self {
        Monotonic { inner: 0.into() }
    }

    pub(crate) fn next(&self) -> usize {
        self.inner.fetch_add(1, Ordering::Relaxed)
    }
}
