pub mod auth;
pub mod authenticator;
pub mod cloud;
pub mod config;
pub mod echoer;
pub mod error;
pub mod identity;
pub mod nodes;
pub mod uppercase;
pub mod vault;
pub mod verifier;

mod util;
pub use util::*;

#[cfg(feature = "lmdb")]
pub mod lmdb;

#[macro_use]
extern crate tracing;

#[derive(rust_embed::RustEmbed)]
#[folder = "./static"]
pub(crate) struct StaticFiles;
