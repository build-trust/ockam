mod create;
mod list;
mod revoke;
mod show;
use std::path::PathBuf;

pub use create::CreateCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use clap::{Args, Subcommand};

use crate::{util::api::CloudOpts, CommandGlobalOpts};

use self::revoke::RevokeCommand;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct LeaseCommand {
    #[command(subcommand)]
    subcommand: LeaseSubcommand,

    #[command(flatten)]
    lease_args: LeaseArgs,
}

#[derive(Clone, Debug, Args)]
pub struct LeaseArgs {
    /// Project config file
    #[arg(global = true, long = "project", value_name = "PROJECT_JSON_PATH")]
    project: Option<PathBuf>,

    #[command(flatten)]
    cloud_opts: CloudOpts,
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
        let project_path = match self.lease_args.project {
            Some(p) => p,
            None => {
                let default_project = options
                    .state
                    .projects
                    .default()
                    .expect("A default project or project parameter is required.");

                default_project.path
            }
        };

        let identity = self.lease_args.cloud_opts.identity;
        match self.subcommand {
            LeaseSubcommand::Create(c) => c.run(options, identity, project_path),
            LeaseSubcommand::List(c) => c.run(options, identity, project_path),
            LeaseSubcommand::Show(c) => c.run(options, identity, project_path),
            LeaseSubcommand::Revoke(c) => c.run(options, identity, project_path),
        }
    }
}
