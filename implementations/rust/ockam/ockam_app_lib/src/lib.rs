//!
//! This crate implements the business logic of the Ockam desktop application without providing a
//! frontend.
//!
//! It exposes C APIs that can be used by the frontend to interact with the application.
//!

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

/// This is a temporary workaround until the fixes done
/// in https://github.com/launchbadge/sqlx/pull/3298 are released
extern crate sqlx_etorreborre as sqlx;
