use crate::util::{node_rpc, parse_node_name};
use crate::{docs, fmt_log, CommandGlobalOpts};
use crate::{
    fmt_ok,
    node::{get_node_name, initialize_node_if_default},
};
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::nodes::models::transport::{CreateTcpListener, TransportStatus};
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use ockam_multiaddr::proto::{DnsAddr, Tcp};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a TCP listener
#[derive(Args, Clone, Debug)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.at);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&node_name)?;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let transport_status: TransportStatus = node
        .ask(
            &ctx,
            Request::post("/node/tcp/listener").body(CreateTcpListener::new(cmd.address)),
        )
        .await?;

    let socket = transport_status.socket_addr().into_diagnostic()?;
    let port = socket.port();
    let mut multiaddr = MultiAddr::default();
    multiaddr
        .push_back(DnsAddr::new("localhost"))
        .into_diagnostic()?;
    multiaddr.push_back(Tcp::new(port)).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(
            fmt_ok!("Tcp listener created! You can send messages to it via this route:\n")
                + &fmt_log!("{multiaddr}"),
        )
        .write_line()?;

    Ok(())
}
