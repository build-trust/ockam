pub mod parser;
mod runner;

use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "run command".to_string()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let config = match &self.inline {
            Some(config) => config.to_string(),
            None => {
                let path = match &self.recipe {
                    Some(path) => path.clone(),
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
        ConfigRunner::run_config(ctx, opts, &config).await
    }
}
