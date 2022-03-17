//! A set of Ockam system handlers

mod delivery;
mod ordering;

/// System handler hooks for Ockam pipes
pub mod pipe {
    pub use super::delivery::{ReceiverConfirm, SenderConfirm};
    pub use super::ordering::{ReceiverOrdering, SenderOrdering};
}
