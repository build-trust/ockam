pub mod parser;
mod runner;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
pub use parser::Config;
pub use runner::ConfigRunner;
use std::path::PathBuf;

/// Create nodes given a declarative configuration file
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide())]
pub struct RunCommand {
    /// Path to the recipe file
    #[arg(long, conflicts_with = "inline")]
    pub recipe: Option<PathBuf>,

    /// Inlined recipe contents
    #[arg(long, conflicts_with = "recipe")]
    pub inline: Option<String>,

    /// If true, block until all the created node exits it also
    /// propagate signals to created nodes.
    /// To be used with docker or kubernetes.
    #[arg(long)]
    pub blocking: bool,
}

impl RunCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), rpc, (opts, self));
    }

    pub fn name(&self) -> String {
        "run command".to_string()
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, RunCommand)) -> miette::Result<()> {
    run_impl(ctx, opts, cmd).await
}

async fn run_impl(ctx: Context, opts: CommandGlobalOpts, cmd: RunCommand) -> miette::Result<()> {
    let config = match cmd.inline {
        Some(config) => config,
        None => {
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
                    Try passing the path to the config file with the --recipe flag."
                        ));
                    }
                    path
                }
            };
            std::fs::read_to_string(path).into_diagnostic()?
        }
    };
    runner::ConfigRunner::go(&ctx, opts, &config).await
}
