use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;

use clap::Args;
use miette::{miette, IntoDiagnostic};
use ockam_api::{
    nodes::InMemoryNode, orchestrator::ai_platform::node_service_client::AI_API_BASE_URL_ENV,
};
use ockam_node::Context;

use crate::{docs, node_command::InMemoryNodeCommand, Command, CommandGlobalOpts, Result};

use super::utils::get_api_client;

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// Generate an enrollment ticket for an Ockam AI Agent
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct TicketCommand {
    // === Specific args for the HTTP API endpoint
    /// The Cluster that will be used
    /// If not set, it will be retrieved from the enrolled user data.
    #[arg(long)]
    pub cluster: Option<String>,

    /// The name of the Zone
    #[arg(long)]
    pub zone_name: String,

    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long)]
    pub use_http_api: bool,

    /// The API endpoint of the Ockam AI Platform.
    /// Defaults to `http://localhost:30080`.
    #[arg(long)]
    pub api_endpoint: Option<String>,

    /// Attributes in `key=value` format to be attached to the member. You can specify this option multiple times for multiple attributes
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    pub attributes: Vec<String>,

    /// Name of the relay that the identity using the ticket will be allowed to create. This name is transformed into attributes to prevent collisions when creating relay names. For example: `--relay foo` is shorthand for `--attribute ockam-relay=foo`
    #[arg(long = "relay", value_name = "ENROLLEE_ALLOWED_RELAY_NAME")]
    pub allowed_relay_name: Option<String>,
}

#[derive(Clone)]
struct TicketNodeCommand {
    opts: CommandGlobalOpts,
    command: TicketCommand,
}

#[async_trait]
impl InMemoryNodeCommand<String> for TicketNodeCommand {
    async fn init(&self) -> miette::Result<()> {
        if let Some(api_endpoint) = &self.command.api_endpoint {
            std::env::set_var(AI_API_BASE_URL_ENV, api_endpoint);
        }
        Ok(())
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<String> {
        let ctx = node.ctx();
        let use_http_api = self.command.use_http_api || self.command.api_endpoint.is_some();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = match &self.command.cluster {
            None => api_client.get_cluster(ctx).await?.into_inner(),
            Some(cluster) => cluster.to_string(),
        };
        let relay = match &self.command.allowed_relay_name {
            Some(relay) => relay.to_string(),
            None => format!(
                "{}-{}-{}",
                cluster,
                self.command.zone_name,
                self.command.zone_name // TODO: review name schema, implementations/rust/ockam/ockam_command/src/cluster/inlet.rs:107
            ),
        };
        let ticket = api_client
            .create_enrollment_token(
                ctx,
                &cluster,
                &self.command.zone_name,
                self.command.attributes()?,
                Some(relay),
            )
            .await?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(format!("\n{ticket}"))
            .machine(&ticket)
            .json(&serde_json::to_string(&ticket).into_diagnostic()?)
            .write_line()?;
        Ok(ticket)
    }
}

#[async_trait]
impl Command<String> for TicketCommand {
    const NAME: &'static str = "cluster ticket";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<String> {
        let command = TicketNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        let ticket = command.execute(ctx, opts.state.clone()).await?;
        Ok(ticket)
    }
}

impl TicketCommand {
    //a bit of copy-pasted from project ticket command. But no tls, enroller, etc.
    fn attributes(&self) -> Result<BTreeMap<String, String>> {
        let mut attributes = BTreeMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            // If no value is provided we assume that the attribute is a boolean attribute set to "true"
            let value = parts.next().unwrap_or("true");
            attributes.insert(key.to_string(), value.to_string());
        }
        Ok(attributes)
    }
}
