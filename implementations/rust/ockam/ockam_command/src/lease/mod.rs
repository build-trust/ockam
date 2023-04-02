mod create;
mod list;
mod revoke;
mod show;

pub use create::CreateCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use clap::{Args, Subcommand};

use crate::{
    util::api::{CloudOpts, ProjectOpts, TrustContextOpts},
    CommandGlobalOpts,
};

use self::revoke::RevokeCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    project_opts: ProjectOpts,

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
            LeaseSubcommand::Create(c) => c.run(
                options,
                self.cloud_opts,
                self.project_opts,
                self.trust_context_opts,
            ),
            LeaseSubcommand::List(c) => c.run(
                options,
                self.cloud_opts,
                self.project_opts,
                self.trust_context_opts,
            ),
            LeaseSubcommand::Show(c) => c.run(
                options,
                self.cloud_opts,
                self.project_opts,
                self.trust_context_opts,
            ),
            LeaseSubcommand::Revoke(c) => c.run(
                options,
                self.cloud_opts,
                self.project_opts,
                self.trust_context_opts,
            ),
        }
    }
}
