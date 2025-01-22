//! This crate provides the ockam command line application to:
//!  - start Ockam nodes and interact with them
//!  - manage projects and spaces hosted within the Ockam Orchestrator
//!
//! For more information please visit the [command guide](https://docs.ockam.io/reference/command)
//!
//! ## Instructions on how to install Ockam Command
//! 1. You can install Ockam Command pre-built binary using these [steps](https://docs.ockam.io/#quick-start). You can run the following command in your terminal to install the pre-built binary:
//!
//!     ```bash
//!     curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
//!     ```
//!
//! 1. To build Ockam Command from source, fork the [repo](https://github.com/build-trust/ockam), and then clone it to your machine. Open a terminal and go to the folder that you just cloned the repo into. Then run the following to install `ockam` so that you can run it from the command line.
//!
//!     ```bash
//!     cd implementations/rust/ockam/ockam_command && cargo install --path .
//!     ```

pub use arguments::*;
pub use command::*;
pub use command_events::*;
pub use command_global_opts::*;
pub use error::*;
pub use global_args::*;
pub use pager::*;
pub use subcommand::*;
pub use terminal::*;

mod admin;
mod arguments;
mod authority;
mod branding;
mod command;
mod command_events;
mod command_global_opts;
mod completion;
mod credential;
mod docs;
pub mod enroll;
pub mod entry_point;
mod environment;
pub mod error;
mod flow_control;
mod global_args;
pub mod identity;
mod influxdb;
mod kafka;
mod lease;
mod manpages;
mod markdown;
mod message;
mod migrate_database;
pub mod node;
mod operation;
mod output;
pub mod pager;
mod policy;
mod project;
mod project_admin;
mod project_member;
mod relay;
mod rendezvous;
mod reset;
mod run;
mod secure_channel;
mod service;
mod share;
mod shared_args;
mod sidecar;
mod space;
mod space_admin;
mod status;
mod subcommand;
mod subscription;
pub mod tcp;
mod terminal;
mod upgrade;
pub mod util;
pub mod value_parsers;
mod vault;
mod version;
mod worker;
