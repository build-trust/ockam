//! Flow Control
//!
//! Allows limiting messaging between workers for security reasons.
//!
//! Flow Control implies 3 roles:
//!
//! - Producers, that produce messages (that usually originate on other nodes), which are
//!     potentially malicious and should be limited in terms of which workers they can reach.
//!     Tcp Receiver is an examples of a Producer.
//! - Consumers, that are allowed to consume potentially malicious messages from Producers.
//!     Secure Channel Decryptor is an example of Consumer.
//! - Spawners, that spawn Consumers and/or Producers
//!     Tcp Listener is an example of Spawner that spawns Producers
//!     Secure Channel Listener is an example of Spawner that spawns both Producers and Consumers.
//!
//! Each Flow Control is identified by a unique random [`FlowControlId`].
//! Producers, Consumers and Spawners are identified by their messaging [`Address`].
//! [`FlowControls`] object is used to store all Flow Control-related data, as well as setup interactions
//! between Producers, Consumers and Spawners.

mod access_control;
mod flow_control_id;
mod flow_controls;
mod policy;

pub use access_control::*;
pub use flow_control_id::*;
pub use flow_controls::*;
pub use policy::*;
