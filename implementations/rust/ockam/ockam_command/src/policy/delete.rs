use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;
use std::str::FromStr;

use ockam::Context;
use ockam_abac::{Action, ResourceName, ResourceType};
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};
use ockam_core::AsyncTryClone;

use super::resource_type_parser;
use crate::terminal::tui::DeleteCommandTui;
use crate::terminal::{color_primary, PluralTerm};
use crate::util::async_cmd;
use crate::{fmt_ok, CommandGlobalOpts, Terminal, TerminalStream};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,

    #[arg(long, conflicts_with = "resource", value_parser = resource_type_parser)]
    resource_type: Option<ResourceType>,

    #[arg(long)]
    resource: Option<ResourceName>,

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
    resource: Option<ResourceTypeOrName>,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        let resource = if cmd.resource_type.is_none() && cmd.resource.is_none() {
            None
        } else {
            Some(
                ResourceTypeOrName::new(cmd.resource_type.as_ref(), cmd.resource.as_ref())
                    .into_diagnostic()?,
            )
        };
        let tui = Self {
            ctx: ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            node,
            cmd,
            resource,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Policy;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.resource.clone().map(|r| r.to_string())
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
            .list_policies(&self.ctx, self.resource.as_ref())
            .await?
            .resource_policies()
            .iter()
            .map(|i| i.resource_name.to_string())
            .collect();
        items_names.extend(
            self.node
                .list_policies(&self.ctx, self.resource.as_ref())
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
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Policy for resource {} has been deleted",
                color_primary(resource.to_string())
            ))
            .json(serde_json::json!({
                "resource": resource.to_string(),
                "at": &self.node.node_name()}
            ))
            .write_line()?;
        Ok(())
    }
}
