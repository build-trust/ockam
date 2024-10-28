//!
//! This crate implements the business logic of the Ockam desktop application without providing a
//! frontend.
//!
//! It exposes C APIs that can be used by the frontend to interact with the application.
//!
#![recursion_limit = "256"]

use thiserror::Error;
mod api;
mod background_node;
mod cli;
mod enroll;
mod error;
mod incoming_services;
mod invitations;
mod log;
mod projects;
mod scheduler;
mod shared_service;
mod state;

pub use error::{Error, Result};
