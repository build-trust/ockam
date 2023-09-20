use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::services::ServiceList;
use ockam_api::DefaultAddress;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{docs, fmt_err, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Kafka Producers
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
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let mut rpc = Rpc::background(&ctx, &opts.state, &node_name).await?;
    let services: ServiceList = rpc
        .ask(Request::get(format!(
            "/node/services/{}",
            DefaultAddress::KAFKA_PRODUCER
        )))
        .await?;
    if services.list.is_empty() {
        opts.terminal
            .stdout()
            .plain(fmt_err!("No Kafka Producers found on this node"))
            .write_line()?;
    } else {
        let mut buf = String::new();
        buf.push_str("Kafka Producers:\n");
        for service in services.list {
            buf.push_str(&format!("{:2}Address: {}\n", "", service.addr));
        }
        opts.terminal.stdout().plain(buf).write_line()?;
    }
    Ok(())
}
