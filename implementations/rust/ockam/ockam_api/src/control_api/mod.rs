//! This module contains the implementation for the Control API and its protocol.
//!
//! The structures are completely independent, so changes outside this module cannot break
//! API compatibility.

pub mod backend;
pub mod frontend;
mod http;
mod openapi;
mod protocol;

pub use openapi::generate_schema;
