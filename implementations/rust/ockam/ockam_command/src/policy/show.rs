use std::str::FromStr;

use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;

use crate::CommandGlobalOpts;
use ockam::Context;
use ockam_abac::{Action, ResourceType};
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::models::policies::ResourceTypeOrName;
use ockam_api::nodes::{BackgroundNodeClient, Policies};
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::TryClone;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    resource: Option<ResourceTypeOrName>,

    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "policy show".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
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
        let tui = Self {
            ctx: ctx.try_clone().into_diagnostic()?,
            opts,
            node,
            resource: cmd.resource,
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
        let resource_kind = match resource {
            ResourceTypeOrName::Type(_) => "resource type",
            ResourceTypeOrName::Name(_) => "resource",
        };
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Policy for {resource_kind} {} is {}",
                color_primary(policy.resource().to_string()),
                color_primary(policy.expression().to_string())
            ))
            .json(serde_json::to_string(&policy).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
