use clap::Args;
use colorful::Colorful;

use ockam_api::nodes::models::services::ServiceList;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::util::node_rpc;
use crate::{docs, fmt_err, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Kafka Consumers
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let services: ServiceList = node
        .ask(
            &ctx,
            Request::get(format!("/node/services/{}", DefaultAddress::KAFKA_DIRECT)),
        )
        .await?;
    if services.list.is_empty() {
        opts.terminal
            .stdout()
            .plain(fmt_err!("No Kafka Direct Client found on this node"))
            .write_line()?;
    } else {
        let mut buf = String::new();
        buf.push_str("Kafka Direct Clients:\n");
        for service in services.list {
            buf.push_str(&format!("{:2}Address: {}\n", "", service.addr));
        }
        opts.terminal.stdout().plain(buf).write_line()?;
    }
    Ok(())
}
