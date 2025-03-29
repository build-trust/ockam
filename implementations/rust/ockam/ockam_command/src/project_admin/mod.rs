mod add;
mod delete;
mod list;

use clap::{Args, Subcommand};

use crate::project_admin::add::AddCommand;
use crate::project_admin::delete::DeleteCommand;
use crate::project_admin::list::ListCommand;
use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), arg_required_else_help = true, subcommand_required = true,
about = docs::about("Manage Project Admins in Ockam Orchestrator"))]
pub struct ProjectAdminCommand {
    #[command(subcommand)]
    subcommand: ProjectAdminSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum ProjectAdminSubcommand {
    #[command(display_order = 800)]
    Add(AddCommand),

    #[command(display_order = 800)]
    List(ListCommand),

    #[command(display_order = 800)]
    Delete(DeleteCommand),
}

impl ProjectAdminCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ProjectAdminSubcommand::List(c) => c.run(ctx, opts).await,
            ProjectAdminSubcommand::Add(c) => c.run(ctx, opts).await,
            ProjectAdminSubcommand::Delete(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ProjectAdminSubcommand::List(c) => c.name(),
            ProjectAdminSubcommand::Add(c) => c.name(),
            ProjectAdminSubcommand::Delete(c) => c.name(),
        }
    }
}
