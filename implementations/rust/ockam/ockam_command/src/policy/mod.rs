use std::str::FromStr;

use clap::{Args, Subcommand};
use miette::miette;

pub use crate::policy::create::CreateCommand;
use crate::policy::delete::DeleteCommand;
use crate::policy::list::ListCommand;
use crate::policy::show::ShowCommand;
use crate::{Command, CommandGlobalOpts};

use ockam_abac::ResourceType;
use ockam_node::Context;

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct PolicyCommand {
    #[command(subcommand)]
    pub subcommand: PolicySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PolicySubcommand {
    #[command(display_order = 900)]
    Create(CreateCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl PolicySubcommand {
    pub fn name(&self) -> String {
        match &self {
            PolicySubcommand::Create(c) => c.name(),
            PolicySubcommand::Show(c) => c.name(),
            PolicySubcommand::Delete(c) => c.name(),
            PolicySubcommand::List(c) => c.name(),
        }
    }
}

impl PolicyCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            PolicySubcommand::Create(c) => c.run(ctx, opts).await,
            PolicySubcommand::Show(c) => c.run(ctx, opts).await,
            PolicySubcommand::Delete(c) => c.run(ctx, opts).await,
            PolicySubcommand::List(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

pub(crate) fn resource_type_parser(input: &str) -> miette::Result<ResourceType> {
    ResourceType::from_str(input).map_err(|_| {
        let valid_values = ResourceType::join_enum_values_as_string();
        miette!(format!("Valid values are: {valid_values}"))
    })
}
