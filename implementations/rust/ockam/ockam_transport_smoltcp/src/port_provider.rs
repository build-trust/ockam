/// A `PortProvider` represents an strategy to get a new port.
///
/// # Example
/// ```
/// use ockam_transport_smoltcp::{PortProvider, ThreadLocalPortProvider};
/// let port = <ThreadLocalPortProvider as PortProvider>::next_port();
/// ```
pub trait PortProvider {
    /// Gets the next available port.
    // Might want to make this method take a `&self` but for now that added too much clutter and for now it's not needed.
    fn next_port() -> u16;
}

#[cfg(feature = "std")]
use rand::Rng;

/// [PortProvider] using [rand::thread_rng].
///
/// It provides a random port between [4096, 65535]. Nothing ensures that there are no collisions.
#[cfg(feature = "std")]
pub struct ThreadLocalPortProvider;

#[cfg(feature = "std")]
impl PortProvider for ThreadLocalPortProvider {
    fn next_port() -> u16 {
        const MIN_PORT: u16 = 4096;
        const MAX_PORT: u16 = 65535;
        rand::thread_rng().gen_range(MIN_PORT..=MAX_PORT)
    }
}
