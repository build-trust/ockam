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
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::{color_primary, PluralTerm};
use crate::util::async_cmd;
use crate::{fmt_ok, CommandGlobalOpts, Terminal, TerminalStream};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,

    #[arg(long = "resource-type", conflicts_with = "resource", value_parser = resource_type_parser)]
    resource_type: Option<ResourceType>,

    #[arg(long)]
    resource: Option<ResourceName>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show policy".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(ctx, opts, self.clone()).await
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    resource: Option<ResourceTypeOrName>,
}

impl ShowTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
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
            resource,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Policy;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.resource.clone().map(|r| r.to_string())
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        Ok(self
            .resource
            .clone()
            .map(|r| r.to_string())
            .unwrap_or(ResourceType::TcpOutlet.to_string()))
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

    async fn show_single(&self, resource: &str) -> miette::Result<()> {
        let resource = if let Ok(resource_type) = ResourceType::from_str(resource) {
            ResourceTypeOrName::Type(resource_type)
        } else {
            ResourceTypeOrName::Name(resource.into())
        };
        let policy = self
            .node
            .show_policy(&self.ctx, &resource, &Action::HandleMessage)
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Policy for resource {} is {}",
                color_primary(policy.resource().to_string()),
                color_primary(policy.expression().to_string())
            ))
            .write_line()?;
        Ok(())
    }
}
