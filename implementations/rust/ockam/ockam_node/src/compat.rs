#[cfg(feature = "std")]
pub mod asynchronous {
    pub use tokio::sync::Mutex;
    pub use tokio::sync::RwLock;
}

#[cfg(not(feature = "std"))]
mod mutex;
#[cfg(not(feature = "std"))]
mod rwlock;

/// async Mutex and RwLock
#[cfg(not(feature = "std"))]
pub mod asynchronous {
    pub use super::mutex::Mutex;
    pub use super::rwlock::RwLock;
}
