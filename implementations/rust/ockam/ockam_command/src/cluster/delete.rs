use crate::cluster::utils::get_api_client;
use crate::node_command::InMemoryNodeCommand;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL_ENV;
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
    /// The name of the Zone to deploy in the Ockam AI Platform
    #[arg(long)]
    pub zone_name: String,

    // === Specific args for the HTTP API endpoint
    /// The Cluster that will be used to set up the Zone.
    /// If not set, it will be retrieved from the enrolled user data.
    #[arg(long)]
    pub cluster: Option<String>,

    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long)]
    pub use_http_api: bool,

    /// The API endpoint of the Ockam AI Platform.
    /// Defaults to `http://localhost:30080`.
    #[arg(long)]
    pub api_endpoint: Option<String>,
}

#[derive(Clone)]
struct DeployNodeCommand {
    opts: CommandGlobalOpts,
    command: DeleteCommand,
}

#[async_trait]
impl InMemoryNodeCommand for DeployNodeCommand {
    async fn init(&self) -> miette::Result<()> {
        if let Some(api_endpoint) = &self.command.api_endpoint {
            std::env::set_var(AI_API_BASE_URL_ENV, api_endpoint);
        }
        Ok(())
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.use_http_api || self.command.api_endpoint.is_some();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = match &self.command.cluster {
            None => api_client.get_cluster(ctx).await?.into_inner(),
            Some(cluster) => cluster.to_string(),
        };
        api_client
            .delete_zone(ctx, &cluster, &self.command.zone_name)
            .await?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(fmt_ok!(
                "Zone {} deleted successfully",
                self.command.zone_name
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
