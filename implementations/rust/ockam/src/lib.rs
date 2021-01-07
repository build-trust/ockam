#![no_std]

#[macro_use]
extern crate alloc;

pub use ockam_macros::*;

#[derive(Debug)]
pub enum Error {
    WorkerRuntime,
}

pub type Result<T> = core::result::Result<T, Error>;

pub mod address;
pub mod queue;
pub mod worker;
