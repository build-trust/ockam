#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

mod net;
mod port_provider;
mod transport;

// Tap devices only make sense in std
#[cfg(feature = "std")]
pub use crate::net::TunTapDevice;

#[cfg(feature = "std")]
pub use port_provider::ThreadLocalPortProvider;

#[cfg(feature = "std")]
pub use net::StdClock;

pub use net::{Clock, Device, Instant, InterfaceConfiguration, StackFacade};
pub use ockam_transport_core::TCP;
pub use port_provider::PortProvider;
pub use transport::*;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.smoltcp";
