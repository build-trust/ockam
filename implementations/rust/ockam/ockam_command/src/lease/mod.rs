mod create;
mod list;
mod revoke;
mod show;

pub use create::CreateCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use clap::{Args, Subcommand};
use miette::{miette, Context, IntoDiagnostic};

use ockam_api::cli_state::{ProjectConfigCompact, StateDirTrait, StateItemTrait};
use ockam_api::cloud::ProjectNode;
use ockam_api::config::lookup::ProjectLookup;
use ockam_api::nodes::Credentials;

use crate::identity::get_identity_name;
use crate::node::util::LocalNode;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::CommandGlobalOpts;

use self::revoke::RevokeCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_context_opts: TrustContextOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum LeaseSubcommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
    Revoke(RevokeCommand),
}

const TOKEN_VIEW: &str = r#"
### Token
> **ID:** ${id}
> **Issued For:** ${issued_for}
> **Created At:** ${created_at}
> **Expires At:** ${expires_at}
> **Token:** ${token}
> **Status:** ${status}
"#;

impl LeaseCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(options, self.cloud_opts, self.trust_context_opts),
            LeaseSubcommand::List(c) => c.run(options, self.cloud_opts, self.trust_context_opts),
            LeaseSubcommand::Show(c) => c.run(options, self.cloud_opts, self.trust_context_opts),
            LeaseSubcommand::Revoke(c) => c.run(options, self.cloud_opts, self.trust_context_opts),
        }
    }
}

async fn authenticate(
    ctx: &ockam_node::Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    trust_opts: &TrustContextOpts,
) -> miette::Result<ProjectNode> {
    let node = LocalNode::create(ctx, opts, Some(trust_opts)).await?;
    let identity = get_identity_name(&opts.state, &cloud_opts.identity);
    let project_info = retrieve_project_info(opts, trust_opts).await?;
    let project_authority = project_info
        .authority
        .as_ref()
        .ok_or(miette!("Project Authority is required"))?;
    let project_identifier = project_info
        .identity_id
        .ok_or(miette!("Project identifier is required"))?;
    let project_addr = project_info
        .node_route
        .ok_or(miette!("Project route is required"))?;

    let authority_node = node
        .make_authority_node_client(
            project_authority.identity_id(),
            project_authority.address(),
            Some(identity.clone()),
        )
        .await?;

    authority_node
        .authenticate(ctx, Some(identity.clone()))
        .await?;
    node.make_project_node_client(&project_identifier, &project_addr, Some(identity.clone()))
        .await
}

async fn retrieve_project_info(
    opts: &CommandGlobalOpts,
    trust_context_opts: &TrustContextOpts,
) -> miette::Result<ProjectLookup> {
    let project_path = match &trust_context_opts.project_path {
        Some(p) => p.clone(),
        None => {
            let default_project = opts
                .state
                .projects
                .default()
                .context("A default project or project parameter is required")?;

            default_project.path().clone()
        }
    };
    // Read (okta and authority) project parameters from project.json
    let s = tokio::fs::read_to_string(project_path)
        .await
        .into_diagnostic()?;
    let proj_info: ProjectConfigCompact = serde_json::from_str(&s).into_diagnostic()?;
    ProjectLookup::from_project(&(&proj_info).into())
        .await
        .into_diagnostic()
}
