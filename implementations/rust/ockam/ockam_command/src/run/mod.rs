use std::path::PathBuf;

use clap::Args;
use miette::{IntoDiagnostic, miette};
use miette::Context as _;

use ockam::Context;

use crate::{CommandGlobalOpts, docs};
use crate::util::node_rpc;

mod parser;

/// Create nodes given a declarative configuration file
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct RunCommand {
    /// Path to the configuration file
    #[arg(long)]
    pub config_path: Option<PathBuf>,
}

impl RunCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, RunCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(_ctx: &Context, opts: CommandGlobalOpts, cmd: RunCommand) -> miette::Result<()> {
    let path = match cmd.config_path {
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
    parser::ConfigRunner::go(&opts.state, &path)?;
    Ok(())
}
