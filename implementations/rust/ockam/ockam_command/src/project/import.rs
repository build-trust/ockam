use clap::ArgGroup;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::Project;

use crate::util::node_rpc;
use crate::{docs, fmt_err, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/import/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/import/after_long_help.txt");

/// Import projects
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
#[clap(group(ArgGroup::new("detailed").required(false)))]
pub struct ImportCommand {
    /// Project file
    #[arg(long, value_name = "PATH")]
    pub project_file: String,
}

impl ImportCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ImportCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: ImportCommand,
) -> miette::Result<()> {
    let file_content = std::fs::read_to_string(&cmd.project_file).into_diagnostic()?;
    let project: Project = serde_json::from_str(&file_content).into_diagnostic()?;
    let result = opts.state.store_project(project.clone()).await;

    match result {
        Ok(_) => opts
            .terminal
            .stdout()
            .plain(fmt_ok!("Successfully imported project {}", &project.name))
            .write_line()?,
        Err(e) => opts
            .terminal
            .stdout()
            .plain(fmt_err!(
                "The project {} could not be imported: {}",
                &cmd.project_file,
                e.to_string()
            ))
            .write_line()?,
    };
    Ok(())
}
