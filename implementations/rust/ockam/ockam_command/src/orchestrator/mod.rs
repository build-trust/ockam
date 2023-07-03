pub(crate) mod show;

use crate::docs;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

use show::ShowCommand;

#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct OrchestratorCommand {
    #[command(subcommand)]
    subcommand: OrchestratorSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum OrchestratorSubcommand {
    Show(ShowCommand),
}

impl OrchestratorCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            OrchestratorSubcommand::Show(c) => c.run(options),
        }
    }
}
