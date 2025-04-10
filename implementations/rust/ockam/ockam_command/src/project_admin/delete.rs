use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_api::orchestrator::project::{Project, ProjectsOrchestratorApi};
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::TryClone;
use std::sync::Arc;

/// Delete an Admin from a Project
#[derive(Clone, Debug, Args)]
#[command()]
pub struct DeleteCommand {
    /// Email of the Admin to delete
    #[arg(value_parser = EmailAddress::parse)]
    email: Option<EmailAddress>,

    /// Name of the Project
    name: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(long, short)]
    yes: bool,

    #[arg(long)]
    all: bool,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "project-admin delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        DeleteNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

#[derive(Clone)]
struct DeleteNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

impl DeleteNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: DeleteCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for DeleteNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let project = self.get_project(node.clone()).await?;
        let tui = DeleteTui {
            opts: self.opts.clone(),
            node: node.clone(),
            command: self.command.clone(),
            project: project.clone(),
        };
        tui.delete().await
    }
}

#[derive(TryClone)]
pub struct DeleteTui {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
    node: Arc<InMemoryNode>,
    project: Project,
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::ProjectAdmin;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.command.email.as_ref().map(|e| e.to_string())
    }

    fn cmd_arg_delete_all(&self) -> bool {
        self.command.all
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.command.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .node
            .list_project_admins(self.project.project_id())
            .await?
            .into_iter()
            .map(|a| a.email)
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.node
            .delete_project_admin(
                self.project.project_id(),
                &EmailAddress::parse(item_name).into_diagnostic()?,
            )
            .await?;
        self.terminal()
            .to_stdout()
            .plain(fmt_ok!(
                "Admin with email {} has been deleted from project {}",
                color_primary(item_name),
                color_primary(self.project.project_name())
            ))
            .machine(item_name)
            .json(serde_json::json!({ "email": item_name }))
            .write_line()?;
        Ok(())
    }
}
