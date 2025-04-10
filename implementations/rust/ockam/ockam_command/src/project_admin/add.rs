use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use std::sync::Arc;

/// Add a new Admin to a Project
#[derive(Clone, Debug, Args)]
#[command()]
pub struct AddCommand {
    /// Email of the Admin to add
    #[arg(value_parser = EmailAddress::parse)]
    email: EmailAddress,

    /// Name of the Project
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[derive(Clone)]
struct AddNodeCommand {
    opts: CommandGlobalOpts,
    command: AddCommand,
}

impl AddNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: AddCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for AddNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project = self.get_project(node.clone()).await?;
        let admin = node
            .add_project_admin(project.project_id(), &self.command.email)
            .await?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(fmt_ok!(
                "Email {} added as an admin to project {}",
                color_primary(self.command.email.to_string()),
                color_primary(project.project_name())
            ))
            .machine(admin.email.to_string())
            .json_obj(admin)?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for AddCommand {
    const NAME: &'static str = "project-admin add";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        AddNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
