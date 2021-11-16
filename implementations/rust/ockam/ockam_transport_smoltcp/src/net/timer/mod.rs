use smoltcp::time::Instant as SmolInstant;

/// Type returned by the [Clock] trait.
///
/// The instant represent elapsed time since some arbitrary starting point and this can be converted into [SmolInstant].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Instant {
    /// Elapsed time measured in milliseconds.
    pub millis: i64,
}

impl From<Instant> for SmolInstant {
    fn from(inst: Instant) -> Self {
        Self::from_millis(inst.millis)
    }
}

impl From<SmolInstant> for Instant {
    fn from(inst: SmolInstant) -> Self {
        Self {
            millis: inst.millis(),
        }
    }
}

/// Monotonic clock trait.
///
/// Used by [SmolTcpTransport](crate::SmolTcpTransport) to provide an instant that will be passed onto the Stack for polling.
///
/// No need to implement this trait if you plan to poll the stack manually.
/// # Example
/// ```
/// use ockam_transport_smoltcp::{Clock, StdClock};
/// let insntant = Clock::now(&StdClock);
/// ```
pub trait Clock {
    /// Returns the elapsed time
    fn now(&self) -> Instant;
}

#[cfg(feature = "std")]
/// `std` implementation of [Clock] using [SmolInstant].
#[derive(Clone, Copy, Debug)]
pub struct StdClock;

#[cfg(feature = "std")]
impl Clock for StdClock {
    fn now(&self) -> Instant {
        SmolInstant::now().into()
    }
}
