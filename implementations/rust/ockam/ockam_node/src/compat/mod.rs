#[cfg(feature = "std")]
/// Async Mutex and RwLock
pub mod asynchronous {
    pub use tokio::sync::Mutex;
    pub use tokio::sync::RwLock;
}

#[cfg(not(feature = "std"))]
mod mutex;
#[cfg(not(feature = "std"))]
mod rwlock;

/// Async Mutex and RwLock
#[cfg(not(feature = "std"))]
pub mod asynchronous {
    pub use super::mutex::Mutex;
    pub use super::rwlock::RwLock;
}
