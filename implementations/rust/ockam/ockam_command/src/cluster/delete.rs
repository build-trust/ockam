use crate::cluster::common_args::{HttpApiArgs, ZoneNameOrConfigArg};
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::node_command::InMemoryNodeCommand;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Deletes an Ockam AI Zone
#[derive(Clone, Debug, Args)]
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
}

#[derive(Clone)]
struct DeployNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

#[async_trait]
impl InMemoryNodeCommand for DeployNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let zone_name = self.command.zone.zone_name()?;
        let spinner = self.opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!("Deleting zone {}...", color_primary(&zone_name)));
        }
        api_client.delete_zone(ctx, &cluster, &zone_name).await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(fmt_ok!(
                "Zone {} deleted successfully",
                color_primary(&zone_name)
            ))
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "cluster delete";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = DeployNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
