use anyhow::Context;
use clap::Args;
use std::path::PathBuf;
use tracing::error;

use crate::{help, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// Run a node given a configuration file
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct RunCommand {
    pub config: PathBuf,
}

impl RunCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = self.run_impl(options) {
            error!(%e);
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }

    fn run_impl(self, _opts: CommandGlobalOpts) -> crate::Result<()> {
        crate::node::util::run::CommandsRunner::run(self.config)
            .context("Failed to run commands from config")?;
        Ok(())
    }
}
