use clap::{Args, Subcommand};
use miette::IntoDiagnostic;

pub use create::CreateCommand;
pub use list::ListCommand;
use ockam_api::cloud::{CredentialsEnabled, ProjectNodeClient};
use ockam_api::nodes::InMemoryNode;
pub use show::ShowCommand;

use crate::shared_args::{IdentityOpts, TrustOpts};
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
    identity_opts: IdentityOpts,

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(opts, self.identity_opts, self.trust_opts),
            LeaseSubcommand::List(c) => c.run(opts, self.identity_opts, self.trust_opts),
            LeaseSubcommand::Show(c) => c.run(opts, self.identity_opts, self.trust_opts),
            LeaseSubcommand::Revoke(c) => c.run(opts, self.identity_opts, self.trust_opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            LeaseSubcommand::Create(c) => c.name(),
            LeaseSubcommand::List(c) => c.name(),
            LeaseSubcommand::Show(c) => c.name(),
            LeaseSubcommand::Revoke(c) => c.name(),
        }
    }
}

async fn create_project_client(
    ctx: &ockam_node::Context,
    opts: &CommandGlobalOpts,
    identity_opts: &IdentityOpts,
    trust_opts: &TrustOpts,
) -> miette::Result<ProjectNodeClient> {
    let node = InMemoryNode::start_with_project_name_and_identity(
        ctx,
        &opts.state,
        identity_opts.identity_name.clone(),
        trust_opts.project_name.clone(),
    )
    .await?;

    let identity = opts
        .state
        .get_identity_name_or_default(&identity_opts.identity_name)
        .await?;
    let project = opts
        .state
        .projects()
        .get_project_by_name_or_default(&trust_opts.project_name)
        .await?;

    node.create_project_client(
        &project.project_identifier().into_diagnostic()?,
        project.project_multiaddr().into_diagnostic()?,
        Some(identity.clone()),
        CredentialsEnabled::On,
    )
    .await
}
