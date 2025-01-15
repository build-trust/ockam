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
        Ok(DeleteTui::run(ctx, opts, self).await?)
    }
}

#[derive(TryClone)]
pub struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: Arc<InMemoryNode>,
    cmd: DeleteCommand,
    project: Project,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let project = opts
            .state
            .projects()
            .get_project_by_name_or_default(&cmd.name)
            .await?;
        let node = InMemoryNode::start_with_identity_and_project_name(
            ctx,
            &opts.state,
            cmd.identity_opts.identity_name.clone(),
            Some(project.project_name().to_string()),
        )
        .await?;
        let tui = Self {
            ctx: ctx.try_clone()?,
            opts,
            node: Arc::new(node),
            cmd,
            project,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::ProjectAdmin;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.email.as_ref().map(|e| e.to_string())
    }

    fn cmd_arg_delete_all(&self) -> bool {
        self.cmd.all
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .node
            .list_project_admins(&self.ctx, self.project.project_id())
            .await?
            .into_iter()
            .map(|a| a.email)
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        self.node
            .delete_project_admin(
                &self.ctx,
                self.project.project_id(),
                &EmailAddress::parse(item_name).into_diagnostic()?,
            )
            .await?;
        self.terminal()
            .stdout()
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
