use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use std::fmt::Write;

use colorful::Colorful;
use ockam_api::address::extract_address_value;
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;
use ockam_node::Context;

use crate::docs;
use crate::node::util::initialize_default_node;
use crate::{Command, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a TCP Connection
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
        let node = BackgroundNodeClient::create(ctx, opts.state.clone(), &self.from).await?;
        let payload = models::transport::CreateTcpConnection::new(self.address.clone());
        let request = Request::post("/node/tcp/connection").body(payload);
        let res: TransportStatus = node.ask(ctx, request).await?;

        opts.terminal
            .to_stdout()
            .plain(self.plain_output(&res, node.node_name())?)
            .machine(res.worker_address.to_string())
            .json_obj(&res)?
            .write_line()?;
        Ok(())
    }
}

impl CreateCommand {
    fn plain_output(&self, status: &TransportStatus, node_name: &str) -> crate::Result<String> {
        let mut plain = String::new();
        writeln!(
            plain,
            "{}",
            fmt_ok!(
                "A TCP {} Connection with worker address {}",
                status.tm,
                color_primary(&status.worker_address),
            ),
        )
        .into_diagnostic()?;
        writeln!(
            plain,
            "{}",
            fmt_log!("was created at the Node {}", color_primary(node_name)),
        )
        .into_diagnostic()?;
        writeln!(
            plain,
            "{}",
            fmt_log!(
                "bound to {}",
                color_primary(status.socket_address.to_string())
            )
        )
        .into_diagnostic()?;
        Ok(plain)
    }
}
