use crate::shared_args::IdentityOpts;
use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
use ockam_api::cloud::email_address::EmailAddress;
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::terminal::{ConfirmResult, Terminal, TerminalStream};
use ockam_api::{fmt_ok, fmt_warn};

/// Delete an Admin from a Space
#[derive(Clone, Debug, Args)]
#[command()]
pub struct DeleteCommand {
    /// Email of the Admin to delete
    #[arg(value_parser = EmailAddress::parse)]
    email: Option<EmailAddress>,

    /// Name of the Space
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
    const NAME: &'static str = "space-admin delete";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(DeleteTui::run(ctx, opts, self).await?)
    }
}

pub struct DeleteTui<'a> {
    ctx: &'a Context,
    opts: CommandGlobalOpts,
    node: InMemoryNode,
    cmd: DeleteCommand,
    space: Space,
    identity_enrolled_email: Option<EmailAddress>,
}

impl<'a> DeleteTui<'a> {
    pub async fn run(
        ctx: &'a Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let space = opts.state.get_space_by_name_or_default(&cmd.name).await?;
        let node = InMemoryNode::start_with_identity(
            ctx,
            &opts.state,
            cmd.identity_opts.identity_name.clone(),
        )
        .await?;
        let identity_name = opts
            .state
            .get_identity_name_or_default(&cmd.identity_opts.identity_name)
            .await?;
        let identity_enrollment = opts
            .state
            .get_identity_enrollment(&identity_name)
            .await?
            .ok_or(miette!("The identity {identity_name} is not enrolled"))?;
        if !identity_enrollment.status().is_enrolled() {
            return Err(miette!("The identity {identity_name} is not enrolled"));
        }

        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
            space,
            identity_enrolled_email: identity_enrollment.status().email().cloned(),
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl<'a> DeleteCommandTui for DeleteTui<'a> {
    const ITEM_NAME: PluralTerm = PluralTerm::SpaceAdmin;

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
            .list_space_admins(self.ctx, &self.space.space_id())
            .await?
            .into_iter()
            .map(|a| a.email)
            .collect())
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        // Prevent the user from deleting the admin that is attached to the enrolled identity.
        // This operation is allowed only if the user confirms the deletion.
        // In non-TTY, it will skip this email.
        if let Some(identity_enrolled_email) = &self.identity_enrolled_email {
            if identity_enrolled_email.to_string() == item_name {
                self.opts.terminal.write_line(fmt_warn!(
                    "You are about to delete {}, which is attached to one of the enrolled identities",
                    color_primary(item_name)
                ))?;
                match self
                    .opts
                    .terminal
                    .confirm("Are you sure you want to delete this admin?")?
                {
                    ConfirmResult::No | ConfirmResult::NonTTY => return Ok(()), // Skip the deletion
                    ConfirmResult::Yes => {} // Continue with the deletion
                }
            }
        }
        self.node
            .delete_space_admin(
                self.ctx,
                &self.space.space_id(),
                &EmailAddress::parse(item_name).into_diagnostic()?,
            )
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Admin with email {} has been deleted from space {}",
                color_primary(item_name),
                color_primary(self.space.space_name())
            ))
            .machine(item_name)
            .json(serde_json::json!({ "email": item_name }))
            .write_line()?;
        Ok(())
    }
}
