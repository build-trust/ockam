use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{docs, fmt_ok, node::NodeOpts, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use ockam_api::nodes::models;
use ockam_core::api::Request;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Kafka Producer
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Kafka producer service address
    pub address: String,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let req = Request::delete("/node/services/kafka_producer").body(
        models::services::DeleteServiceRequest::new(cmd.address.clone()),
    );
    rpc.request(req).await?;
    rpc.is_ok()?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Kafka producer with address `{}` successfully deleted",
            cmd.address
        ))
        .write_line()?;

    Ok(())
}
