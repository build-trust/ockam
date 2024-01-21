use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::transport::{CreateTcpListener, TransportStatus};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_multiaddr::proto::{DnsAddr, Tcp};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use crate::{fmt_log, fmt_ok};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a TCP listener
#[derive(Args, Clone, Debug)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node at which to create the listener
    #[arg(global = true, long, value_name = "NODE", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Address for this listener (eg. 127.0.0.1:7000)
    pub address: String,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
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
