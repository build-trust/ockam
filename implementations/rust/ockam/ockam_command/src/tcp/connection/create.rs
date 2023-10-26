use clap::Args;
use indoc::formatdoc;
use miette::IntoDiagnostic;

use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_node::Context;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::{
    docs,
    util::{api, node_rpc},
    CommandGlobalOpts,
};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct TcpConnectionNodeOpts {
    /// Node that will initiate the connection
    #[arg(global = true, short, long, value_name = "NODE")]
    pub from: Option<String>,
}

/// Create a TCP connection
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: TcpConnectionNodeOpts,

    /// The address to connect to
    #[arg(id = "to", short, long, value_name = "ADDRESS")]
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.from);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let from = get_node_name(&opts.state, &cmd.node_opts.from);
    let node_name = extract_address_value(&from)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let request = api::create_tcp_connection(&cmd);
    let transport_status: TransportStatus = node.ask(&ctx, request).await?;
    let to = transport_status.socket_addr().into_diagnostic()?;
    let plain = formatdoc! {r#"
        TCP Connection:
            From: /node/{from}
            To: {to} (/ip4/{}/tcp/{})
            Address: {}
    "#, to.ip(), to.port(), transport_status.multiaddr().into_diagnostic()?};
    let json = serde_json::json!([{"route": transport_status.multiaddr().into_diagnostic()? }]);
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
