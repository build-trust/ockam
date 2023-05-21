mod parser;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Result};
use anyhow::Context as _;
use clap::Args;
use miette::miette;
use ockam::Context;
use std::path::PathBuf;

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

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, RunCommand)) -> Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(_ctx: &Context, opts: CommandGlobalOpts, cmd: RunCommand) -> Result<()> {
    let path = match cmd.config_path {
        Some(path) => path,
        None => {
            let mut path = std::env::current_dir().context("Failed to get current directory")?;
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
                )
                .into());
            }
            path
        }
    };
    parser::ConfigRunner::go(&opts.state, &path)?;
    Ok(())
}
