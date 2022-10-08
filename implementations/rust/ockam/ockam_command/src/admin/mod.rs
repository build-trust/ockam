use clap::{Args, Subcommand};

use crate::util::api::CloudOpts;
use crate::{help, CommandGlobalOpts};

mod subscription;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), after_long_help = help::template(HELP_DETAIL))]
pub struct AdminCommand {
    #[command(subcommand)]
    pub subcommand: AdminSubCommand,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AdminSubCommand {
    #[command(display_order = 800)]
    Subscription(subscription::SubscriptionCommand),
}

impl AdminCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            AdminSubCommand::Subscription(c) => c.run(options),
        }
    }
}
