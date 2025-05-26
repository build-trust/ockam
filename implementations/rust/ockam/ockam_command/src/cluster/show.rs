use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::node_command::InMemoryNodeCommand;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use serde::Serialize;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Shows your Cluster ID
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[derive(Clone)]
struct DeployNodeCommand {
    opts: CommandGlobalOpts,
    command: ShowCommand,
}

#[async_trait]
impl InMemoryNodeCommand for DeployNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let zones = api_client.list_zones(ctx, Some(&cluster)).await?;
        let output = ShowOutput {
            cluster: cluster.clone(),
            zones: zones.clone(),
        };
        let mut plain = fmt_ok!("Your cluster is {}\n", color_primary(&cluster));
        if !zones.is_empty() {
            plain += &fmt_log!(
                "with zones: {}",
                zones
                    .iter()
                    .map(|z| color_primary(z).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .machine(&cluster)
            .json_obj(output)?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "cluster show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = DeployNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

#[derive(Serialize)]
struct ShowOutput {
    cluster: String,
    zones: Vec<String>,
}
