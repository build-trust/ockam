mod delete;
mod get;
mod list;
mod set;
use crate::policy::delete::DeleteCommand;
use crate::policy::get::GetCommand;
use crate::policy::list::ListCommand;
use crate::policy::set::SetCommand;
use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};
use ockam_abac::{Action, Resource};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), after_long_help = help::template(HELP_DETAIL))]
pub struct PolicyCommand {
    #[command(subcommand)]
    subcommand: PolicySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PolicySubcommand {
    Set(SetCommand),
    Get(GetCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl PolicyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            PolicySubcommand::Set(c) => c.run(opts),
            PolicySubcommand::Get(c) => c.run(opts),
            PolicySubcommand::Delete(c) => c.run(opts),
            PolicySubcommand::List(c) => c.run(opts),
        }
    }
}

fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}
