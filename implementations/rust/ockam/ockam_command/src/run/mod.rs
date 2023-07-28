mod parser;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
pub use parser::ConfigRunner;
use std::path::PathBuf;

/// Create nodes given a declarative configuration file
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct RunCommand {
    /// Path to the recipe file
    #[arg(long)]
    pub recipe: Option<PathBuf>,
    /// If true, block until all the created node exits it also
    /// propagate signals to created nodes.
    /// To be used with docker or kubernetes.
    #[arg(long)]
    pub blocking: bool,
}

impl RunCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(_ctx: Context, (opts, cmd): (CommandGlobalOpts, RunCommand)) -> miette::Result<()> {
    run_impl(opts, cmd).await
}

async fn run_impl(opts: CommandGlobalOpts, cmd: RunCommand) -> miette::Result<()> {
    let path = match cmd.recipe {
        Some(path) => path,
        None => {
            let mut path = std::env::current_dir()
                .into_diagnostic()
                .context("Failed to get current directory")?;
            let default_file_names = ["ockam.yml", "ockam.yaml"];
            let mut found = false;
            for file_name in default_file_names.iter() {
                path.push(file_name);
                if path.exists() {
                    found = true;
                    break;
                }
                path.pop();
            }
            if !found {
                return Err(miette!(
                    "No default configuration file found in current directory.\n\
                    Try passing the path to the config file with the --config-path flag."
                ));
            }
            path
        }
    };
    ConfigRunner::go(opts, &path, cmd.blocking).await
}
