use clap::{Args, Subcommand};

use crate::shared_args::IdentityOpts;
use crate::{docs, CommandGlobalOpts};

use ockam_node::Context;

mod subscription;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), after_long_help = docs::after_help(HELP_DETAIL))]
pub struct AdminCommand {
    #[command(subcommand)]
    pub subcommand: AdminSubCommand,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AdminSubCommand {
    #[command(display_order = 800)]
    Subscription(subscription::SubscriptionCommand),
}

impl AdminCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            AdminSubCommand::Subscription(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            AdminSubCommand::Subscription(c) => c.name(),
        }
    }
}
