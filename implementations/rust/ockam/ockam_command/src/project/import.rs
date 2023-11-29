use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;

use ockam::identity::{Identifier, Identity};
use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::util::node_rpc;
use crate::util::parsers::{identity_identifier_parser, multiaddr_parser, validate_project_name};
use crate::{docs, fmt_err, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/import/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/import/after_long_help.txt");

/// Import projects
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ImportCommand {
    /// Project name
    #[arg(long, value_parser = validate_project_name)]
    pub project_name: String,

    /// Project id
    #[arg(long)]
    pub project_id: String,

    /// Project identifier
    #[arg(long, value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub project_identifier: Option<Identifier>,

    /// Project access route
    #[arg(long, value_name = "MULTIADDR", value_parser = multiaddr_parser)]
    pub project_access_route: MultiAddr,

    /// Hex encoded Identity
    #[arg(long, value_name = "IDENTITY")]
    authority_identity: Option<String>,

    /// Authority access route
    #[arg(long, value_name = "MULTIADDR", value_parser = multiaddr_parser)]
    pub authority_access_route: Option<MultiAddr>,
}

impl ImportCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    pub async fn authority_identity(&self) -> miette::Result<Option<Identity>> {
        match &self.authority_identity {
            Some(i) => Ok(Some(Identity::create(i).await.into_diagnostic()?)),
            None => Ok(None),
        }
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
    match opts
        .state
        .import_project(
            &cmd.project_id,
            &cmd.project_name,
            &cmd.project_identifier,
            &cmd.project_access_route,
            &cmd.authority_identity().await?,
            &cmd.authority_access_route,
        )
        .await
    {
        Ok(_) => opts
            .terminal
            .stdout()
            .plain(fmt_ok!(
                "Successfully imported project {}",
                &cmd.project_name
            ))
            .write_line()?,
        Err(e) => opts
            .terminal
            .stdout()
            .plain(fmt_err!(
                "The project {} could not be imported: {}",
                &cmd.project_name,
                e.to_string()
            ))
            .write_line()?,
    };
    Ok(())
}
