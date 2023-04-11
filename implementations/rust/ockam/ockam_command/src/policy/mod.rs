mod create;
mod delete;
mod list;
mod show;
use crate::policy::create::CreateCommand;
use crate::policy::delete::DeleteCommand;
use crate::policy::list::ListCommand;
use crate::policy::show::ShowCommand;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use ockam_abac::{Action, Resource};

#[derive(Clone, Debug, Args)]
pub struct PolicyCommand {
    #[command(subcommand)]
    subcommand: PolicySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PolicySubcommand {
    #[command(display_order = 900)]
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl PolicyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            PolicySubcommand::Create(c) => c.run(opts),
            PolicySubcommand::Show(c) => c.run(opts),
            PolicySubcommand::Delete(c) => c.run(opts),
            PolicySubcommand::List(c) => c.run(opts),
        }
    }
}

fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}
