use ockam_api::cloud::project::Project;

mod commands;
mod events;
pub(super) mod plugin;

pub(crate) type State = Vec<Project>;
