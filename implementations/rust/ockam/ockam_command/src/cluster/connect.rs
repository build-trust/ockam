use async_trait::async_trait;
use std::sync::Arc;

use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node_command::InMemoryNodeCommand;
use crate::tcp::inlet::create::tcp_inlet_default_from_addr;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::parsers::hostname_parser;
use crate::{docs, Command, CommandGlobalOpts, Result};
use clap::Args;
use ockam::transport::SchemeHostnamePort;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/connect/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/connect/after_long_help.txt");

/// Connect to a service provided by an Ockam AI Agent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ConnectCommand {
    /// The Cluster that hosts the Zone.
    #[arg(long)]
    pub cluster: Option<String>,

    /// The name of the Zone to connect to
    #[arg(long)]
    pub zone_name: String,

    /// References the name of TCP Outlet created in the Zone and the Relay name.
    #[arg(long)]
    pub pod: String,

    // == Node Options ==
    #[arg(long, env = "ENROLLMENT_TICKET", value_name = "ENROLLMENT TICKET")]
    #[arg(help = docs::about("\
    A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node. \
    When passed, the identity will be given a project membership credential. \
    Check the `project ticket` command for more information about enrollment tickets.
    "))]
    pub enrollment_ticket: String,

    // == TCP Inlet Options ==
    /// Address on which to accept TCP connections, in the format `<scheme>://<host>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default host is `127.0.0.1`.
    /// If the argument is not set, a random port will be used on the default address `tcp://127.0.0.1`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = tcp_inlet_default_from_addr(), value_parser = hostname_parser)]
    pub from: SchemeHostnamePort,

    // === Specific args for the HTTP API endpoint
    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long)]
    pub use_http_api: bool,
}

#[derive(Clone)]
struct ConnectNodeCommand {
    opts: CommandGlobalOpts,
    command: ConnectCommand,
}

#[async_trait]
impl InMemoryNodeCommand for ConnectNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let api_client = get_api_client(&node, self.command.use_http_api).await?;
        let cluster = api_client
            .get_cluster(node.ctx(), &self.command.zone_name)
            .await?;
        let relay_name = format!(
            "{}-{}-{}",
            cluster, self.command.zone_name, self.command.pod
        );
        let node_config = serde_json::json!({
            "tcp-inlet": {
                "from": self.command.from.to_string(),
                "to": self.command.pod,
                "via": relay_name
            }
        });
        let node_cmd = crate::node::create::CreateCommand {
            name: node_config.to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(self.command.enrollment_ticket.clone()),
                ..Default::default()
            },
            foreground_args: ForegroundArgs {
                foreground: true,
                ..Default::default()
            },
            ..Default::default()
        };
        node_cmd.run(node.ctx(), self.opts.clone()).await
    }
}

#[async_trait]
impl Command for ConnectCommand {
    const NAME: &'static str = "cluster connect";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = ConnectNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
