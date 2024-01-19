use clap::{command, Args, Subcommand};
use ockam_api::cloud::account::Accounts;
use ockam_api::cloud::project::Projects;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

use crate::{
    output::CredentialAndPurposeKeyDisplay,
    util::{api::CloudOpts, node_rpc},
    CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct AccountCommand {
    #[command(subcommand)]
    subcommand: AccountSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AccountSubcommand {
    Credential(GetAccountCredentialCommand),
}

#[derive(Clone, Debug, Args)]
pub struct GetAccountCredentialCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,

    #[arg(long, value_name = "PROJECT_NAME")]
    pub project: Option<String>,
}

impl GetAccountCredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, GetAccountCredentialCommand),
) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: GetAccountCredentialCommand,
) -> miette::Result<()> {
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let credential = if let Some(project_name) = cmd.project {
        let project = node.get_project_by_name(ctx, &project_name).await?;
        node.get_project_admin_credential(ctx, &project.id).await?
    } else {
        node.get_account_credential(ctx).await?
    };
    opts.terminal
        .clone()
        .stdout()
        .plain(CredentialAndPurposeKeyDisplay(credential))
        .write_line()?;
    Ok(())
}

impl AccountCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            AccountSubcommand::Credential(c) => c.run(options),
        }
    }
}
