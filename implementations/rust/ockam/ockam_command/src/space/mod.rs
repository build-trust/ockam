use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct SpaceCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl SpaceCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: SpaceCommand) {
        match cmd.subcommand {
            SpaceSubcommand::Create(scmd) => {
                CreateCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            SpaceSubcommand::Delete(scmd) => {
                DeleteCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            SpaceSubcommand::List(scmd) => {
                ListCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            SpaceSubcommand::Show(scmd) => {
                ShowCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
        }
    }
}
