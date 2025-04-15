use async_trait::async_trait;

use crate::ai::utils::get_customer_name;
use crate::node::config::ConfigArgs;
use crate::tcp::inlet::create::tcp_inlet_default_from_addr;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::parsers::hostname_parser;
use crate::{docs, Command, CommandGlobalOpts, Result};
use clap::Args;
use ockam::transport::SchemeHostnamePort;
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
    /// The name of the Customer that owns the Zone.
    #[arg(long)]
    pub customer: String,

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
}

#[async_trait]
impl Command for ConnectCommand {
    const NAME: &'static str = "ai connect";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let customer_name = get_customer_name(&opts, Some(&self.customer)).await?;
        let relay_name = format!("{}-{}-{}", customer_name, self.zone_name, self.pod);
        let node_config = serde_json::json!({
            "tcp-inlet": {
                "from": self.from.to_string(),
                "to": self.pod,
                "via": relay_name
            }
        });
        let node_cmd = crate::node::create::CreateCommand {
            name: node_config.to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(self.enrollment_ticket),
                ..Default::default()
            },
            foreground_args: ForegroundArgs {
                foreground: true,
                ..Default::default()
            },
            ..Default::default()
        };
        node_cmd.run(ctx, opts).await
    }
}
