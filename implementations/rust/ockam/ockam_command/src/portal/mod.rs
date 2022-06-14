mod create;
// mod delete;
// mod list;

pub(crate) use create::CreateCommand;
// pub(crate) use delete::DeleteCommand;
// use list::ListCommand;

use crate::{util::OckamConfig, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct PortalCommand {
    #[clap(subcommand)]
    subcommand: PortalSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PortalSubCommand {
    /// Create portals on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    // /// Delete portals on the selected node
    // #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    // Delete(DeleteCommand),

    // /// List portals registered on the selected node
    // #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    // List(ListCommand),
}
