//! ockam_node - Ockam Node API
/* Copyright 2021 {Authors}
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*/
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

pub use context::*;
pub use error::*;
pub use executor::*;
pub use node::*;
pub use worker::*;

mod context;
mod error;
mod executor;
mod node;
mod worker;

/// A unique identifier for entities in the Ockam Node.
pub type Address = String;

/// Top level [`Context`] and [`NodeExecutor`] for async main initialization.
pub fn node<T>() -> (Context, NodeExecutor) {
    let executor = NodeExecutor::new();
    let context = executor.new_worker_context("node");
    (context, executor)
}
