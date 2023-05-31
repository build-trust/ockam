use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};
use anyhow::Context;
use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam_api::nodes::models;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_core::api::Request;

const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP connections
#[derive(Args, Clone, Debug)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.api_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.api_node);
    let node_name = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(Request::get("/node/tcp/connection")).await?;
    let response = rpc.parse_response::<models::transport::TransportList>()?;

    let table = response
        .list
        .iter()
        .fold(
            vec![],
            |mut acc,
             TransportStatus {
                 tt,
                 tm,
                 socket_addr,
                 worker_addr,
                 processor_address,
                 flow_control_id,
                 ..
             }| {
                let row = vec![
                    tt.cell(),
                    tm.cell(),
                    socket_addr.cell(),
                    worker_addr.cell(),
                    processor_address.cell(),
                    flow_control_id.cell(),
                ];
                acc.push(row);
                acc
            },
        )
        .table()
        .title(vec![
            "Type".cell().bold(true),
            "Mode".cell().bold(true),
            "Socket address".cell().bold(true),
            "Worker address".cell().bold(true),
            "Processor address".cell().bold(true),
            "Flow Control Id".cell().bold(true),
        ]);

    print_stdout(table).context("failed to print node status")?;
    Ok(())
}
