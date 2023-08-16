use ockam_api::cloud::project::Project;

mod commands;
pub(crate) mod events;
pub(super) mod plugin;

pub(crate) type State = Vec<Project>;
