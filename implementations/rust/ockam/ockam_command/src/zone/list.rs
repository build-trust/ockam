use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::show::ShowOutput;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::{docs, node_command::InMemoryNodeCommand, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List zones
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
    command: ListCommand,
}

#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;

        let spinner = self.opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message("Retrieving zones...");
        }

        let cluster = get_cluster(ctx, &node).await?;
        let zones = api_client.list_zones(ctx, Some(&cluster)).await?;

        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }

        let output = ShowOutput {
            cluster: cluster.clone(),
            zones: zones.clone(),
        };
        let plain = if !zones.is_empty() {
            fmt_ok!("Your cluster {}\n", color_primary(&cluster))
                + &fmt_log!(
                    "has the zones: {}",
                    zones
                        .iter()
                        .map(|z| color_primary(z).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
        } else {
            fmt_log!("Your cluster {} has no zones", color_primary(&cluster))
        };
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
impl Command for ListCommand {
    const NAME: &'static str = "zone list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = ListNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
