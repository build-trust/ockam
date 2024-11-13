mod processor_relay;
mod worker_relay;

pub use processor_relay::*;
pub use worker_relay::*;

/// A signal type used to communicate between router and worker relay
#[derive(Clone, Debug)]
pub enum CtrlSignal {
    /// Interrupt current message execution and shut down
    InterruptStop,
}
