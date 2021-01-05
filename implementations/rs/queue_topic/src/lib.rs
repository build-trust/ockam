#![no_std]

#[macro_use]
extern crate alloc;
extern crate hashbrown;

use alloc::string::String;

pub mod queue;
pub mod topic;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum QueueError {
    Unknown,
}

pub trait Addressable {
    fn address(&self) -> String;
}
