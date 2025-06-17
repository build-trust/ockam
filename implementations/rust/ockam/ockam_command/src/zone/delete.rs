use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::node_command::InMemoryNodeCommand;
use crate::tui::{DeleteCommandTui, PluralTerm};
use crate::zone::common_args::ZoneNameOrConfigArg;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use console::Term;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::TryClone;
use ockam_node::Context;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a zone
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    /// Delete all zones in the cluster
    #[arg(long)]
    pub all: bool,

    /// Confirm the deletion without prompting
    #[arg(long, short)]
    pub yes: bool,
}

#[derive(Clone)]
struct DeleteNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

#[async_trait]
impl InMemoryNodeCommand for DeleteNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        DeleteTui::run(
            ctx,
            self.opts.clone(),
            self.command.clone(),
            api_client,
            cluster,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "zone delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = DeleteNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

#[derive(TryClone)]
struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
    api_client: Arc<Box<dyn AiPlatformApi + Send + Sync + 'static>>,
    cluster: String,
}

impl DeleteTui {
    pub async fn run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
        api_client: Box<dyn AiPlatformApi + Send + Sync + 'static>,
        cluster: String,
    ) -> miette::Result<()> {
        let tui = Self {
            ctx: ctx.try_clone()?,
            opts,
            cmd,
            api_client: Arc::new(api_client),
            cluster,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Zone;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.zone.zone_name().ok()
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
        // Only remove the zone defined in the config if `all` is not set.
        if !self.cmd.all {
            if let Ok(zone_config) = self.cmd.zone.zone_config() {
                return Ok(vec![zone_config.name]);
            }
        }
        // Otherwise, list all zones in the cluster to let the user choose which ones to delete.
        self.api_client
            .list_zones(&self.ctx, Some(&self.cluster))
            .await
    }

    async fn delete_single(&self, item_name: &str) -> miette::Result<()> {
        let spinner = self.opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!("Deleting zone {}...", color_primary(item_name)));
        }
        self.api_client
            .delete_zone(&self.ctx, Some(&self.cluster), item_name)
            .await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        self.terminal()
            .to_stdout()
            .plain(fmt_ok!(
                "Zone {} deleted successfully",
                color_primary(item_name)
            ))
            .write_line()?;
        Ok(())
    }
}
