#![no_std]

extern crate alloc;

// re-export the #[node] attribute macro.
pub use ockam_node_attribute::*;

#[derive(Debug)]
pub enum Error {
    WorkerRuntime,
}

pub type Result<T> = core::result::Result<T, Error>;

pub mod address;
pub mod queue;
pub mod node;
pub mod worker;
