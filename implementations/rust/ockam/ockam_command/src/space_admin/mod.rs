mod add;
mod delete;
mod list;

use clap::{Args, Subcommand};

use crate::space_admin::add::AddCommand;
use crate::space_admin::delete::DeleteCommand;
use crate::space_admin::list::ListCommand;
use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true,
about = docs::about("Manage Space Admins in Ockam Orchestrator"))]
pub struct SpaceAdminCommand {
    #[command(subcommand)]
    subcommand: SpaceAdminSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum SpaceAdminSubcommand {
    #[command(display_order = 800)]
    Add(AddCommand),

    #[command(display_order = 800)]
    List(ListCommand),

    #[command(display_order = 800)]
    Delete(DeleteCommand),
}

impl SpaceAdminCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SpaceAdminSubcommand::List(c) => c.run(ctx, opts).await,
            SpaceAdminSubcommand::Add(c) => c.run(ctx, opts).await,
            SpaceAdminSubcommand::Delete(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SpaceAdminSubcommand::List(c) => c.name(),
            SpaceAdminSubcommand::Add(c) => c.name(),
            SpaceAdminSubcommand::Delete(c) => c.name(),
        }
    }
}
