use clap::{Args, Subcommand};

use crate::util::api::CloudOpts;
use crate::{help, CommandGlobalOpts};

mod subscription;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), help_template = help::template(HELP_DETAIL))]
pub struct AdminCommand {
    #[clap(subcommand)]
    pub subcommand: AdminSubCommand,

    #[clap(flatten)]
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
