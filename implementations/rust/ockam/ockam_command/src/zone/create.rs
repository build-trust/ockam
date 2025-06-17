use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::node_command::InMemoryNodeCommand;
use crate::zone::common_args::ZoneNameOrConfigArg;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a zone
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[derive(Clone)]
struct CreateNodeCommand {
    opts: CommandGlobalOpts,
    command: CreateCommand,
}

#[async_trait]
impl InMemoryNodeCommand for CreateNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let zone_name = self.command.zone.zone_name()?;
        api_client
            .create_zone(ctx, Some(&cluster), &zone_name)
            .await?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(fmt_ok!(
                "Created zone {} in cluster {}\n",
                color_primary(&zone_name),
                color_primary(&cluster)
            ))
            .machine(zone_name)
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "zone create";

    async fn run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = CreateNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
