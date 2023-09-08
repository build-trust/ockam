use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::project::ProjectVersion;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/version/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/version/after_long_help.txt");

/// Return the version of the Orchestrator Controller and the Projects
#[derive(Clone, Debug, Args)]
#[command(
    long_about=docs::about(LONG_ABOUT),
    after_long_help=docs::about(AFTER_LONG_HELP)
)]
pub struct VersionCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl VersionCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, options);
    }
}

async fn rpc(mut ctx: Context, opts: CommandGlobalOpts) -> miette::Result<()> {
    run_impl(&mut ctx, opts).await
}

async fn run_impl(ctx: &mut Context, opts: CommandGlobalOpts) -> miette::Result<()> {
    // Send request
    let mut rpc = Rpc::embedded(ctx, &opts).await?;
    let project_version: ProjectVersion = rpc.ask(api::project::version()).await?;
    delete_embedded_node(&opts, rpc.node_name()).await;

    let json = serde_json::to_string(&project_version).into_diagnostic()?;
    let project_version = project_version
        .project_version
        .unwrap_or("unknown".to_string());
    let plain = fmt_ok!("The version of the Projects is '{project_version}'");

    opts.terminal
        .stdout()
        .plain(plain)
        .machine(project_version)
        .json(json)
        .write_line()?;
    Ok(())
}
