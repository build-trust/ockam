//! The ockam_abac crate is responsible for performing attribute based authorization
//! control on messages within an Ockam worker system.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

mod env;
mod error;
mod eval;
mod policy;
mod types;

#[cfg(feature = "std")]
mod parser;

pub mod attribute_access_control;
pub mod expr;
pub mod resource;

pub use attribute_access_control::AbacAccessControl;
pub use env::Env;
pub use error::{EvalError, ParseError};
pub use eval::eval;
pub use expr::Expr;
pub use policy::{storage::*, Policies, PolicyAccessControl, ResourcePolicy, ResourceTypePolicy};
pub use resource::{Resource, ResourceType};
pub use types::{Action, ResourceName, Subject};

#[cfg(feature = "std")]
pub use parser::parse;

#[cfg(not(feature = "std"))]
pub use ockam_executor::tokio;

#[cfg(feature = "std")]
pub use tokio;
