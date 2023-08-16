use ockam_api::cloud::project::Project;

mod commands;
mod error;
pub(crate) mod events;
pub(super) mod plugin;

pub(crate) type State = Vec<Project>;
