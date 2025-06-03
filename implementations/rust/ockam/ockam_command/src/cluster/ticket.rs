use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;

use clap::Args;
use miette::{miette, IntoDiagnostic};
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

use super::utils::{get_api_client, get_cluster};
use crate::cluster::common_args::HttpApiArgs;
use crate::zone::common_args::ZoneNameOrConfigArg;
use crate::{docs, node_command::InMemoryNodeCommand, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/ticket/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/ticket/after_long_help.txt");

/// Generate an enrollment ticket
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct TicketCommand {
    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

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
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<String> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let zone_name = self.command.zone.zone_name()?;
        let ticket = api_client
            .create_enrollment_ticket(
                ctx,
                Some(&cluster),
                &zone_name,
                self.command.attributes()?,
                self.command.allowed_relay_name.clone(),
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
