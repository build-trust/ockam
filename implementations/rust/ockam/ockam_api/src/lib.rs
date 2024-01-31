//! This crate supports the creation of a fully-featured Ockam Node
//! (see [`NodeManager`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs#L87) in [`src/nodes/service.rs`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs)).
//!
//! # Configuration
//!
//! A `NodeManager` maintains its database and log files on disk in
//! the `OCKAM_HOME` directory (`~/.ockam`) by default:
//! ```shell
//! root
//! ├─ database.sqlite
//! ├─ nodes
//! │  ├─ node1
//! │  │  ├─ stderr.log
//! │  │  ├─ stdout.log
//! │  ├─ node2
//! │  └─ ...
//! ```

#[macro_use]
extern crate tracing;

pub mod address;
pub mod authenticator;
pub mod cli_state;
pub mod cloud;
pub mod config;
pub mod echoer;
pub mod enroll;
pub mod error;
pub mod hop;
pub mod kafka;
pub mod minicbor_url;
pub mod nodes;
pub mod okta;
pub mod port_range;
pub mod uppercase;

pub mod authority_node;
mod influxdb_token_lease;

pub mod logs;
mod schema;
mod session;
mod util;

pub use cli_state::*;
pub use influxdb_token_lease::*;
pub use nodes::service::default_address::*;
pub use session::sessions::ConnectionStatus;
pub use util::*;
