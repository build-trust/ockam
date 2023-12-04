use clap::{Args, Subcommand};
use miette::IntoDiagnostic;

pub use create::CreateCommand;
pub use list::ListCommand;
use ockam_api::cloud::{CredentialsEnabled, ProjectNodeClient};
use ockam_api::nodes::InMemoryNode;
pub use show::ShowCommand;

use crate::util::api::{CloudOpts, TrustOpts};
use crate::CommandGlobalOpts;

use self::revoke::RevokeCommand;

mod create;
mod list;
mod revoke;
mod show;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_opts: TrustOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum LeaseSubcommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
    Revoke(RevokeCommand),
}

impl LeaseCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(options, self.cloud_opts, self.trust_opts),
            LeaseSubcommand::List(c) => c.run(options, self.cloud_opts, self.trust_opts),
            LeaseSubcommand::Show(c) => c.run(options, self.cloud_opts, self.trust_opts),
            LeaseSubcommand::Revoke(c) => c.run(options, self.cloud_opts, self.trust_opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            LeaseSubcommand::Create(_) => "create lease",
            LeaseSubcommand::List(_) => "list leases",
            LeaseSubcommand::Show(_) => "show lease",
            LeaseSubcommand::Revoke(_) => "revoke lease",
        }
        .to_string()
    }
}

async fn create_project_client(
    ctx: &ockam_node::Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    trust_opts: &TrustOpts,
) -> miette::Result<ProjectNodeClient> {
    let node = InMemoryNode::start_with_project_name_and_identity(
        ctx,
        &opts.state,
        cloud_opts.identity.clone(),
        trust_opts.project_name.clone(),
    )
    .await?;

    let identity = opts
        .state
        .get_identity_name_or_default(&cloud_opts.identity)
        .await?;
    let project = opts
        .state
        .get_project_by_name_or_default(&trust_opts.project_name)
        .await?;

    node.create_project_client(
        &project.identifier().into_diagnostic()?,
        &project.access_route().into_diagnostic()?,
        Some(identity.clone()),
        CredentialsEnabled::On,
    )
    .await
}
