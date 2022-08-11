use crate::{Mailbox, RelayMessage};

#[cfg(feature = "debugger")]
use core::sync::atomic::{AtomicU32, Ordering};

/// Log incoming message flow authorization
pub fn log_incoming_access_control(_mailbox: &Mailbox, _relay_msg: &RelayMessage) {
    #[cfg(feature = "debugger")]
    {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        tracing::debug!(
            "log_incoming_access_control #{:03}: {:?} for {} -> {}",
            COUNTER.fetch_add(1, Ordering::Relaxed),
            _mailbox.incoming_access_control(),
            &_relay_msg.source,
            &_relay_msg.destination
        );
    }
}

/// Log outgoing message flow authorization
pub fn log_outgoing_access_control(_mailbox: &Mailbox, _relay_msg: &RelayMessage) {
    #[cfg(feature = "debugger")]
    {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        tracing::debug!(
            "log_outgoing_access_control #{:03}: {:?} for {} -> {}",
            COUNTER.fetch_add(1, Ordering::Relaxed),
            _mailbox.outgoing_access_control(),
            &_relay_msg.source,
            &_relay_msg.destination
        );
    }
}
