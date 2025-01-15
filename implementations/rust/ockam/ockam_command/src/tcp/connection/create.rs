use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use serde::Serialize;
use std::fmt::Write;
use std::net::SocketAddrV4;

use colorful::Colorful;
use ockam_api::address::extract_address_value;
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_api::output::Output;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::docs;
use crate::node::util::initialize_default_node;
use crate::{Command, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a TCP connection
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node that will initiate the connection
    #[arg(long, value_name = "NODE", value_parser = extract_address_value)]
    pub from: Option<String>,

    /// The address to connect to
    #[arg(id = "to", short, long, value_name = "ADDRESS")]
    pub address: String,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "tcp-connection create";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.from).await?;
        let payload = models::transport::CreateTcpConnection::new(self.address.clone());
        let request = Request::post("/node/tcp/connection").body(payload);
        let transport_status: TransportStatus = node.ask(ctx, request).await?;

        let output = TcpConnection::new(
            node.node_name().to_string(),
            transport_status.socket_addr().into_diagnostic()?,
            transport_status.multiaddr().into_diagnostic()?,
        );

        opts.terminal
            .stdout()
            .plain(output.item()?)
            .machine(output.address.to_string())
            .json(serde_json::to_string(&output).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct TcpConnection {
    from: String,
    to: SocketAddrV4,
    address: MultiAddr,
}

impl TcpConnection {
    pub fn new(from: String, to: SocketAddrV4, address: MultiAddr) -> Self {
        Self { from, to, address }
    }
}

impl Output for TcpConnection {
    fn item(&self) -> ockam_api::Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "{}",
            fmt_ok!(
                "A TCP connection was created at the node {}",
                color_primary(&self.from)
            ),
        )?;
        writeln!(
            output,
            "{}",
            fmt_log!("to the address {}", color_primary(self.to.to_string()))
        )?;
        Ok(output)
    }
}
