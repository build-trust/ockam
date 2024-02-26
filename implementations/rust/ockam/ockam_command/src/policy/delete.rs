use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;
use std::str::FromStr;

use ockam::Context;
use ockam_abac::{Action, ResourceType};
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};
use ockam_core::AsyncTryClone;

use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::{color_primary, PluralTerm};
use crate::util::async_cmd;
use crate::{fmt_ok, CommandGlobalOpts, Terminal, TerminalStream};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    resource: Option<ResourceTypeOrName>,

    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "delete policy".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        DeleteTui::run(ctx, opts, self.clone()).await
    }
}

struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        let tui = Self {
            ctx: ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Policy;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.resource.clone().map(|r| r.to_string())
    }

    fn cmd_arg_delete_all(&self) -> bool {
        false
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let mut items_names: Vec<String> = self
            .node
            .list_policies(&self.ctx, self.cmd.resource.as_ref())
            .await?
            .resource_policies()
            .iter()
            .map(|i| i.resource_name.to_string())
            .collect();
        items_names.extend(
            self.node
                .list_policies(&self.ctx, self.cmd.resource.as_ref())
                .await?
                .resource_type_policies()
                .iter()
                .map(|i| i.resource_type.to_string()),
        );
        Ok(items_names)
    }

    async fn delete_single(&self, resource: &str) -> miette::Result<()> {
        let resource = if let Ok(resource_type) = ResourceType::from_str(resource) {
            ResourceTypeOrName::Type(resource_type)
        } else {
            ResourceTypeOrName::Name(resource.into())
        };
        self.node
            .delete_policy(&self.ctx, &resource, &Action::HandleMessage)
            .await?;
        let resource_kind = match resource {
            ResourceTypeOrName::Type(_) => "resource type",
            ResourceTypeOrName::Name(_) => "resource",
        };
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Policy for {resource_kind} {} has been deleted",
                color_primary(resource.to_string())
            ))
            .write_line()?;
        Ok(())
    }
}
